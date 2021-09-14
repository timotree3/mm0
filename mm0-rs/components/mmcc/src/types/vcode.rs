//! The low level IR, based on cranelift's `VCode`.

use std::{convert::TryInto, iter::FromIterator};

use crate::{Idx, types::IdxVec};

use mm0_util::u32_as_usize;
use regalloc2::RegallocOptions;
pub(crate) use regalloc2::{
  PReg, VReg, RegClass, InstRange, Operand, Block as BlockId, Inst as InstId};

use super::Size;

impl Idx for BlockId {
  fn into_usize(self) -> usize { self.index() }
  fn from_usize(n: usize) -> Self { Self::new(n) }
}
impl Idx for InstId {
  fn into_usize(self) -> usize { self.index() }
  fn from_usize(n: usize) -> Self { Self::new(n) }
}
mk_id! {
  /// An ID for a constant to be placed in the constant pool.
  ConstId,
  /// An ID for a global static variable to be placed in the global area.
  GlobalId,
  /// An ID for a (monomorphized) function that can be called.
  ProcId,
  /// An ID for a spill slot (a piece of the stack frame)
  SpillId,
}

impl SpillId {
  /// The spill slot corresponding to the incoming arguments from the caller.
  pub const INCOMING: SpillId = SpillId(0);
  /// The spill slot corresponding to the outgoing arguments, to functions called by this one.
  pub const OUTGOING: SpillId = SpillId(1);
}

/// A type for instruction data in a `VCode<I>`.
pub trait Inst: Sized {
  /// Determine whether an instruction is a call instruction. This is used
  /// only for splitting heuristics.
  fn is_call(&self) -> bool;

  /// Determine whether an instruction is a return instruction.
  fn is_ret(&self) -> bool;

  /// Determine whether an instruction is the end-of-block
  /// branch. If so, its operands at the indices given by
  /// `blockparam_arg_offset()` below *must* be the block
  /// parameters for each of its block's `block_succs` successor
  /// blocks, in order.
  fn is_branch(&self) -> bool;

  /// Returns the operand index at which outgoing blockparam arguments are found.
  /// Starting at this index, blockparam arguments for each
  /// successor block's blockparams, in order, must be found.
  fn branch_blockparam_arg_offset(&self) -> usize;

  /// Determine whether an instruction is a move; if so, return the
  /// Operands for (src, dst).
  fn is_move(&self) -> Option<(Operand, Operand)>;

  /// Get the Operands for an instruction.
  fn collect_operands(&self, _: &mut Vec<Operand>);

  /// Get the clobbers for an instruction.
  fn clobbers(&self) -> &[PReg];
}

/// Conceptually the same as `IdxVec<I, Vec<T>>`, but shares allocations between the vectors.
/// Best used for append-only use, since only the last added element can be pushed to.
#[derive(Debug)]
pub struct ChunkVec<I, T> {
  data: Vec<T>,
  idxs: IdxVec<I, u32>,
}

impl<I, T> Default for ChunkVec<I, T> {
  fn default() -> Self { Self { data: vec![], idxs: Default::default() } }
}

impl<I: Idx, T> ChunkVec<I, T> {
  fn push_into(&mut self, f: impl FnOnce(&mut Vec<T>)) -> I {
    let i = self.idxs.push(self.data.len().try_into().expect("overflow"));
    f(&mut self.data);
    i
  }

  fn push(&mut self, it: impl IntoIterator<Item=T>) -> I {
    self.push_into(|v| v.extend(it))
  }

  fn start(&self, i: I) -> usize { u32_as_usize(self.idxs[i]) }
  fn end(&self, i: I) -> usize {
    match self.idxs.0.get(i.into_usize() + 1) {
      None => self.data.len(),
      Some(&b) => u32_as_usize(b)
    }
  }
  fn extent(&self, i: I) -> std::ops::Range<usize> { self.start(i)..self.end(i) }
}

impl<I: Idx, T> std::ops::Index<I> for ChunkVec<I, T> {
  type Output = [T];
  fn index(&self, i: I) -> &[T] { &self.data[self.extent(i)] }
}

impl<I: Idx, T, A: IntoIterator<Item = T>> FromIterator<A> for ChunkVec<I, T> {
  fn from_iter<J: IntoIterator<Item = A>>(iter: J) -> Self {
    let mut out = Self::default();
    for it in iter { out.push(it); }
    out
  }
}

/// The calling convention of a single argument.
#[allow(variant_size_differences)]
#[derive(Clone, Copy, Debug)]
pub(crate) enum ArgAbi {
  /// The value is not passed.
  Ghost,
  /// The value is passed in the given physical register.
  Reg(PReg, Size),
  /// The value is passed in a memory location.
  Mem {
    /// The offset in the `OUTGOING` slot to find the data.
    off: u32,
    /// The size of the data in bytes.
    sz: u32
  },
  /// A pointer to a value of the given size is passed in a physical register.
  /// Note: For return values with this ABI, this is an additional argument *to* the function:
  /// the caller passes a pointer to the return slot.
  Boxed {
    /// The register carrying the pointer.
    reg: PReg,
    /// The size of the pointed-to data in bytes.
    sz: u32
  },
  /// A pointer to the value is passed in memory. This is like `Boxed`,
  /// but for the case that we have run out of physical registers.
  /// (The pointer is at `off..off+8`, and the actual value is at `[off]..[off]+sz`.)
  /// Note: For return values with this ABI, this is an additional argument *to* the function:
  /// the caller puts a pointer to the return slot at this location in the outgoing slot.
  BoxedMem {
    /// The offset in the `OUTGOING` slot to find the pointer. (It has a fixed size of 8.)
    off: u32,
    /// The size of the data starting at the pointer location.
    sz: u32
  },
}

/// The representation of a monomorphized function's calling convention.
#[derive(Debug)]
pub(crate) struct ProcAbi {
  /// The arguments of the procedure.
  pub(crate) args: Box<[ArgAbi]>,
  /// The return values of the procedure. (Functions and procedures return multiple values in MMC.)
  /// If None, then the function does not return.
  pub(crate) rets: Option<Box<[ArgAbi]>>,
  /// The total size of all the outgoing arguments in bytes
  args_space: u32,
  /// The registers that are clobbered by the call.
  pub(crate) clobbers: Box<[PReg]>,
}

/// A low level representation of a function, after instruction selection but before
/// register allocation.
#[derive(Debug)]
pub struct VCode<I> {
  insts: IdxVec<InstId, I>,
  blocks: IdxVec<BlockId, (InstId, InstId)>,
  block_preds: IdxVec<BlockId, Vec<BlockId>>,
  block_succs: IdxVec<BlockId, Vec<BlockId>>,
  block_params: ChunkVec<BlockId, VReg>,
  operands: ChunkVec<InstId, Operand>,
  num_vregs: usize,
  num_spills: usize,
  outgoing_spill_size: Option<u32>,
}

impl<I> Default for VCode<I> {
  fn default() -> Self {
    Self {
      insts: Default::default(),
      blocks: Default::default(),
      block_preds: Default::default(),
      block_succs: Default::default(),
      block_params: Default::default(),
      operands: Default::default(),
      num_vregs: 0,
      num_spills: 2, // INCOMING, OUTGOING
      outgoing_spill_size: None,
    }
  }
}

impl<I> VCode<I> {
  /// Create a new unused `VReg`.
  pub fn fresh_vreg(&mut self) -> VReg {
    let v = VReg::new(self.num_vregs, RegClass::Int);
    self.num_vregs += 1;
    v
  }

  /// Create a new unused `SpillId`.
  pub fn fresh_spill(&mut self) -> SpillId {
    let n = SpillId::from_usize(self.num_spills);
    self.num_spills += 1;
    n
  }

  /// Finalize a block. Must be called after each call to `new_block`,
  /// once all instructions of the block are emitted.
  pub fn finish_block(&mut self) {
    self.blocks.0.last_mut().expect("no blocks").1 = InstId::new(self.insts.len());
  }

  /// Make space in the outgoing argument stack region.
  pub fn mk_outgoing_spill(&mut self, sz: u32) {
    let old = self.outgoing_spill_size.get_or_insert(0);
    *old = (*old).max(sz);
  }

  /// Add an edge in the CFG, from `from` to `to`.
  pub fn add_edge(&mut self, from: BlockId, to: BlockId) {
    self.block_succs[from].push(to);
    self.block_preds[to].push(from);
  }

  /// Start a new block, giving the list of block parameters.
  pub fn new_block(&mut self, params: impl IntoIterator<Item=VReg>) -> BlockId {
    let inst = InstId::new(self.insts.len());
    let bl = self.blocks.push((inst, inst));
    self.block_preds.push(vec![]);
    self.block_succs.push(vec![]);
    self.block_params.push(params);
    bl
  }
}

impl<I: Inst> VCode<I> {
  /// Emit an instruction into the current block.
  pub fn emit(&mut self, inst: I) -> InstId {
    self.operands.push_into(|v| inst.collect_operands(v));
    self.insts.push(inst)
  }
}

impl<I> std::ops::Index<InstId> for VCode<I> {
  type Output = I;
  fn index(&self, i: InstId) -> &Self::Output { &self.insts[i] }
}
impl<I> std::ops::IndexMut<InstId> for VCode<I> {
  fn index_mut(&mut self, i: InstId) -> &mut Self::Output { &mut self.insts[i] }
}

impl<I: Inst> regalloc2::Function for VCode<I> {
  fn num_insts(&self) -> usize { self.insts.len() }
  fn num_blocks(&self) -> usize { self.blocks.len() }
  fn entry_block(&self) -> BlockId { BlockId::new(0) }

  fn block_insns(&self, block: BlockId) -> InstRange {
    let (from, to) = self.blocks[block];
    InstRange::forward(from, to)
  }

  fn block_succs(&self, block: BlockId) -> &[BlockId] { &self.block_succs[block] }
  fn block_preds(&self, block: BlockId) -> &[BlockId] { &self.block_preds[block] }
  fn block_params(&self, block: BlockId) -> &[VReg] { &self.block_params[block] }

  fn is_ret(&self, insn: InstId) -> bool { self.insts[insn].is_ret() }
  fn is_branch(&self, insn: InstId) -> bool { self.insts[insn].is_branch() }

  fn branch_blockparam_arg_offset(&self, _: BlockId, insn: InstId) -> usize {
    self.insts[insn].branch_blockparam_arg_offset()
  }

  fn is_move(&self, insn: InstId) -> Option<(Operand, Operand)> { self.insts[insn].is_move() }
  fn inst_operands(&self, insn: InstId) -> &[Operand] { &self.operands[insn] }
  fn inst_clobbers(&self, insn: InstId) -> &[PReg] { self.insts[insn].clobbers() }
  fn num_vregs(&self) -> usize { self.num_vregs }
  fn spillslot_size(&self, _: regalloc2::RegClass) -> usize { 1 }
}

impl<I: Inst> VCode<I> {
  fn regalloc(&self) -> regalloc2::Output {
    let opts = RegallocOptions { verbose_log: false };
    regalloc2::run(self, &crate::arch::MACHINE_ENV, &opts).expect("fatal regalloc error")
  }
}
