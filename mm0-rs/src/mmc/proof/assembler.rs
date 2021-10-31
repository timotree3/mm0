use mm0_util::{AtomId, FileSpan, Modifiers, Span};
use mmcc::{arch::OpcodeLayout, proof::{AssemblyBlocks, Inst, Proc}};

use super::{ProofDedup, Predefs, ProofId, Dedup, norm_num::{HexCache, Num}};

pub(super) struct BuildAssemblyProc<'a> {
  pub(super) thm: ProofDedup<'a>,
  hex: HexCache,
  start: Num,
}

impl<'a> std::ops::Deref for BuildAssemblyProc<'a> {
  type Target = ProofDedup<'a>;
  fn deref(&self) -> &Self::Target { &self.thm }
}
impl<'a> std::ops::DerefMut for BuildAssemblyProc<'a> {
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.thm }
}

enum DecodeResult {
  Reg,
  Jump { tgt: Num, c: ProofId, q: ProofId },
  Call { tgt: Num, q: ProofId }
}

fn parse_arr<const N: usize>(p: &mut &[u8]) -> [u8; N] {
  let (start, rest) = p.split_at(N);
  *p = rest;
  start.try_into().expect("parse error")
}
fn parse_u8(p: &mut &[u8]) -> u8 { parse_arr::<1>(p)[0] }
fn parse_u32(p: &mut &[u8]) -> u32 { u32::from_le_bytes(parse_arr(p)) }
fn parse_u64(p: &mut &[u8]) -> u64 { u64::from_le_bytes(parse_arr(p)) }

impl BuildAssemblyProc<'_> {
  fn bisect<T>(&mut self,
    n: usize, iter: &mut impl Iterator<Item=T>, x: Num,
    f: &mut impl FnMut(&mut Self, T, Num) -> (ProofId, ProofId, Num, ProofId),
  ) -> (ProofId, ProofId, Num, ProofId) {
    if n <= 1 {
      assert!(n != 0);
      let item = iter.next().expect("iterator size lied");
      f(self, item, x)
    } else {
      let m = n >> 1;
      let (s, a, y, pr1) = self.bisect(m, iter, x, f);
      let (t, b, z, pr2) = self.bisect(n - m, iter, y, f);
      let st = app!(self, (sadd s t));
      let ab = app!(self, (asmp_A a b));
      let pr = thm!(self, is_asmp_A(*self.start, s, t, *x, *y, *z, a, b):
        (is_asmp {*self.start} st {*x} {*z} ab));
      (st, ab, z, pr)
    }
  }

  /// Given a string `s` with `len s < 16`, proves `(i, |- len s = x[i])`
  fn strlen(&mut self, s: ProofId) -> (Num, ProofId) {
    let mut args = vec![];
    let mut x = s;
    loop {
      app_match!(self, x => {
        (scons c x2) => { args.push(c); x = x2 }
        (s1 a) => {
          args.push(a);
          #[allow(clippy::cast_possible_truncation)]
          let n = self.hex.h2n(&mut self.thm, args.len() as u8);
          let res = app!(self, (len s) = {*n});
          let th = self.strlenn[args.len()];
          return (n, self.thm(th, &args, res))
        }
        (s0) => {
          assert!(args.is_empty());
          let n = self.hex.h2n(&mut self.thm, 0);
          return (n, thm!(self, strlenn[0_usize](): (len s) = {*n}))
        }
        _ => panic!("not a string literal"),
      })
    }
  }

  /// Given `x`, proves `(ast, s, |- decode ast s)` where
  /// `s` is generated from the instruction assembly.
  fn decode_aux(&mut self, p: &mut &[u8], layout: OpcodeLayout, rex: ProofId
  ) -> ([ProofId; 4], DecodeResult) {
    let opc = parse_u8(p);
    let (x, y) = (opc >> 4, opc & 15);
    match layout {
      OpcodeLayout::BinopRAX => {
        let ([v, d, _, o], h1) = self.hex.split_bits_1111(&mut self.thm, y);
        let (_, h2) = self.hex.split_bits_22(&mut self.thm, x);
        let opc = (opc >> 3) & 7;
        let (_, h3) = self.hex.split_bits_121(&mut self.thm, opc);
        todo!()
      }
      OpcodeLayout::BinopImmS8(_) => todo!(),
      OpcodeLayout::BinopImmS32(_) => todo!(),
      OpcodeLayout::BinopImm8(_) => todo!(),
      OpcodeLayout::BinopReg(_) => todo!(),
      OpcodeLayout::BinopHi(_) => todo!(),
      OpcodeLayout::BinopHi1(_) => todo!(),
      OpcodeLayout::BinopHiReg(_) => todo!(),
      OpcodeLayout::MovSX(_) => todo!(),
      OpcodeLayout::MovReg(_) => todo!(),
      OpcodeLayout::Mov32 => todo!(),
      OpcodeLayout::Mov64 => todo!(),
      OpcodeLayout::MovImm(_) => todo!(),
      OpcodeLayout::PushImm8 => todo!(),
      OpcodeLayout::PushImm32 => todo!(),
      OpcodeLayout::PushReg => todo!(),
      OpcodeLayout::PopReg => todo!(),
      OpcodeLayout::Jump8 => todo!(),
      OpcodeLayout::Jump32 => todo!(),
      OpcodeLayout::Jcc8 => todo!(),
      OpcodeLayout::Call => todo!(),
      OpcodeLayout::Ret => todo!(),
      OpcodeLayout::Lea(_) => todo!(),
      OpcodeLayout::Test(_) => todo!(),
      OpcodeLayout::TestRAX8 => todo!(),
      OpcodeLayout::TestRAX32 => todo!(),
      OpcodeLayout::Hi(_) => todo!(),
      OpcodeLayout::HiTest8(_) => todo!(),
      OpcodeLayout::HiTest32(_) => todo!(),
      OpcodeLayout::SetCC(_) => todo!(),
      OpcodeLayout::CMov(_) => todo!(),
      OpcodeLayout::MovX(_) => todo!(),
      OpcodeLayout::Jcc => todo!(),
      OpcodeLayout::SysCall => todo!(),
      OpcodeLayout::Assert => todo!(),
      OpcodeLayout::Ud2 => todo!(),
    }
  }

  /// Proves `([ast, s, |- decode ast s], res)` where
  /// `s` is generated from the instruction assembly. `res` contains
  /// additional information about the decode.
  fn decode_inst(&mut self, inst: &Inst<'_>) -> ([ProofId; 3], DecodeResult) {
    let p = &mut inst.content();
    if inst.layout.rex {
      let rex = self.hex[parse_u8(p) & 15];
      let srex = app!(self, (suc (h2n rex)));
      let ([ast, opc, s, pr], r) = self.decode_aux(p, inst.layout.opc, srex);
      let s2 = app!(self, (scons opc s));
      let pr = thm!(self, decode0I(ast, opc, s, pr): (decode ast s2));
      ([ast, s2, pr], r)
    } else {
      let rex = app!(self, (d0));
      let ([ast, opc, s, pr], r) = self.decode_aux(p, inst.layout.opc, rex);
      let s2 = app!(self, (scons (ch {self.hex[4]} rex) (scons opc s)));
      let pr = thm!(self, decode1I(ast, opc, rex, s, pr): (decode ast s2));
      ([ast, s2, pr], r)
    }
  }

  /// Given `x`, proves `(s, A, y, |- is_asmp start s x y A)` where
  /// `s` is generated from the instruction assembly.
  fn asm_inst(&mut self, inst: &Inst<'_>, x: Num) -> (ProofId, ProofId, Num, ProofId) {
    let ([ast, s, h1], res) = self.decode_inst(inst);
    let (n, h2) = self.strlen(s);
    let (y, h3) = self.hex.add(&mut self.thm, x, n);
    let (a, pr) = match res {
      DecodeResult::Reg => {
        let a = app!(self, (asmi ast));
        (a, thm!(self, ((is_asmp {*self.start} s {*x} {*y} a)) =>
          asmiI(*self.start, s, *x, *y, ast, *n, h1, h2, h3)))
      }
      DecodeResult::Jump { tgt, c, q } => {
        let (q2, he) = self.hex.sub64(&mut self.thm, tgt, y);
        assert!(q == *q2);
        let a = app!(self, (asmJCC c {*tgt}));
        (a, thm!(self, ((is_asmp {*self.start} s {*x} {*y} a)) =>
          asmJCC_I(*self.start, s, *x, *y, c, *tgt, q, *n, h1, he, h2, h3)))
      }
      DecodeResult::Call { tgt, q } => {
        let (src, he1) = self.hex.add(&mut self.thm, self.start, x);
        let (q2, he2) = self.hex.sub64(&mut self.thm, tgt, src);
        assert!(q == *q2);
        let a = app!(self, (asmCall {*tgt}));
        (a, thm!(self, ((is_asmp {*self.start} s {*x} {*y} a)) =>
          asmCallI(*self.start, s, *x, *y, *src, *tgt, q, *n, h1, he1, he2, h2, h3)))
      }
    };
    (s, a, y, pr)

  }

  /// Given `x`, proves `(s, A, y, |- is_asmp start s x y A)` where
  /// `s` is generated from the assembly blocks.
  pub(super) fn proc(&mut self, proc: &Proc<'_>) -> (ProofId, ProofId, Num, ProofId) {
    let mut iter = proc.assembly_blocks();
    let x = self.hex.h2n(&mut self.thm, 0);
    self.bisect(iter.len(), &mut iter, x, &mut |this, block, x| {
      let mut iter = block.insts();
      let (s, a, y, pr) = this.bisect(iter.len(), &mut iter, x, &mut |this, inst, x| {
        todo!()
      });
      // axiom is_asmp_at (p s x y A)
      //   (h1: $ is_asmp p s x y A $): $ is_asmp p s x y (asmp_at x A) $;
      let a2 = app!(this.thm, (asmp_at {*x} a));
      let pr = thm!(this.thm, is_asmp_at(*this.start, s, *x, *y, a):
        (is_asmp {*this.start} s {*x} {*y} a2));
      (s, a2, y, pr)
    })
  }
}

pub(super) fn assemble_proc(
  pd: &Predefs,
  name: AtomId,
  proc: &Proc<'_>,
  span: FileSpan,
  full: Span,
) -> crate::Thm {
  let mut thm = ProofDedup::new(pd, &[]);
  let hex = HexCache::new(&mut thm);
  let mut build = BuildAssemblyProc {
    start: hex.from_u64(&mut thm, proc.start()),
    hex,
    thm,
  };
  let pr = build.proc(proc).3;
  build.thm.build_thm0(name, Modifiers::empty(), span, full, pr)
}