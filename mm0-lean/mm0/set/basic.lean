/-#
This file is not autogenerated, but it is meant as a prelude for the
autogenerated set.mm import. To use this file, you should run mm0-hs on set.mm
to generate the MM0 files out.mm0 and out.mmu, and then feed these back into
mm0-hs to generate files setN.lean in this directory.

* The `-a .basic` says that all axioms should be commented out and this
  file should be referenced to find the definitions. (This will cause errors in
  the resulting lean file because this file doesn't yet have every axiom and
  definition in set.mm.)
* The `-c 2000` says to chunk up the output lean file
  every 2000 statements; this helps keep lean performant and allows for
  incremental builds. (You may want to adjust this value smaller, 2000 statements
  means the files are around 30000 lines which is quite high by lean standards.)

```
stack exec -- mm0-hs from-mm set.mm -o out.mm0 out.mmu
stack exec -- mm0-hs to-lean out.mm0 out.mmu -a .basic -c 2000 -o mm0-lean/mm0/set/set.lean
```

-/
import set_theory.zfc

namespace mm0

def wff : Type := Prop
def wff.proof : wff → Prop := id
def wff.forget {p : Prop} (h : wff → p) : p := h true
def wi : wff → wff → wff := (→)
def wn : wff → wff := not

theorem ax_1 {ph ps : wff} : ph → ps → ph := λ h _, h
theorem ax_2 {ph ps ch : wff} :
  (ph → ps → ch) → (ph → ps) → ph → ch := λ h1 h2 h3, h1 h3 (h2 h3)
theorem ax_3 {ph ps : wff} : (¬ ph → ¬ ps) → ps → ph :=
λ h1 h2, classical.by_contradiction $ λ h3, h1 h3 h2
theorem ax_mp {ph ps : wff} : ph → (ph → ps) → ps := λ h1 h2, h2 h1

def wb : wff → wff → wff := iff

theorem df_an {ph ps : wff} : ph ∧ ps ↔ ¬ (ph → ¬ ps) :=
⟨λ ⟨h1, h2⟩ h3, h3 h1 h2, λ h1, classical.by_contradiction $
  λ h2, h1 $ λ h3 h4, h2 ⟨h3, h4⟩⟩

theorem df_bi.aux {ph ps : Prop} : (ph ↔ ps) ↔ ¬((ph → ps) → ¬(ps → ph)) :=
iff_def.trans df_an

theorem df_bi {ph ps : wff} :
  ¬(((ph ↔ ps) → ¬((ph → ps) → ¬(ps → ph))) →
  ¬(¬((ph → ps) → ¬(ps → ph)) → (ph ↔ ps))) :=
df_bi.aux.1 df_bi.aux

def wo : wff → wff → wff := or

theorem df_or {ph ps : wff} : ph ∨ ps ↔ (¬ ph → ps) :=
classical.or_iff_not_imp_left

def wa : wff → wff → wff := and

def w3o (p q r : wff) : wff := p ∨ q ∨ r

def w3a (p q r : wff) : wff := p ∧ q ∧ r

theorem df_3or {ph ps ch : Prop} : ph ∨ ps ∨ ch ↔ (ph ∨ ps) ∨ ch :=
or.assoc.symm

theorem df_3an {a b c : Prop} : a ∧ b ∧ c ↔ (a ∧ b) ∧ c :=
and.assoc.symm

def wnan (ph ps : wff) : wff := ¬ (ph ∧ ps)

theorem df_nan {ph ps : wff} : wb (wnan ph ps) (wn (wa ph ps)) :=
iff.rfl

def wxo : wff → wff → wff := xor

theorem df_xor {ph ps : wff} : xor ph ps ↔ ¬ (ph ↔ ps) :=
⟨λ h1 h2, or.cases_on h1 (λ ⟨h3, h4⟩, h4 (h2.1 h3)) (λ ⟨h3, h4⟩, h4 (h2.2 h3)),
 λ h1, classical.by_contradiction $ λ h2, h1
  ⟨λ h, classical.by_contradiction $ λ hn, h2 (or.inl ⟨h, hn⟩),
   λ h, classical.by_contradiction $ λ hn, h2 (or.inr ⟨h, hn⟩)⟩⟩

def wtru := true
def wfal := false

theorem df_tru {ph : wff} : wb wtru (wb ph ph) :=
(iff_true_intro iff.rfl).symm

theorem df_fal : wb wfal (wn wtru) := not_true_iff.symm

def whad (ph ps ch : wff) : wff := wxo (wxo ph ps) ch

def wcad (ph ps ch : wff) : wff := wo (wa ph ps) (wa ch (wxo ph ps))

theorem df_had {ph ps ch : wff} : wb (whad ph ps ch) (wxo (wxo ph ps) ch) := iff.rfl

theorem df_cad {ph ps ch : wff} : wb (wcad ph ps ch) (wo (wa ph ps) (wa ch (wxo ph ps))) := iff.rfl

theorem ax_meredith {ph ps ch th ta : wff}
  (h1 : (((ph → ps) → ¬ ch → ¬ th) → ch) → ta)
  (h2 : ta → ph) (h3 : th) : ph :=
classical.by_contradiction $ λ hn, hn $ h2 $ h1 $ λ h,
classical.by_contradiction $ λ hc, h (not.elim hn) hc h3

@[reducible] def setvar : Type 1 := Set

def setvar.forget {p : Prop} (h : setvar → p) : p := h (∅ : Set)
def wal (P : setvar → wff) : wff := ∀ x, P x

def wex : (setvar → wff) → wff := Exists

theorem df_ex {ph : setvar → wff} : wb (wex (λ x, ph x)) (wn (wal (λ x, wn (ph x)))) :=
by classical; exact not_forall_not.symm

def wnf (ph : setvar → wff) : wff := ∀ x, ph x → ∀ y, ph y

theorem df_nf {ph : setvar → wff} : wnf ph ↔ ∀ x, ph x → ∀ y, ph y := iff.rfl

theorem ax_gen {ph : setvar → wff} (h : ∀ x, ph x) : ∀ x, ph x := h

theorem ax_4 {ph ps : setvar → wff} (h : ∀ x, ph x → ps x)
  (h2 : ∀ x, ph x) (x) : ps x := h x (h2 x)

theorem ax_5 {ph : wff} (h : ph) (x : setvar) : ph := h

@[reducible] def «class» : Type 1 := Class

def «class».forget {p : Prop} (h : «class» → p) : p := h ∅
def cv : setvar → «class» := Class.of_Set

def wceq : «class» → «class» → wff := eq
local notation x ` ≡ ` y := eq (↑x : «class») ↑y

theorem weq' {x y : setvar} : x ≡ y ↔ x = y := ⟨Class.of_Set.inj, congr_arg _⟩

def wsb (ph : setvar → wff) (y : setvar → setvar) : wff :=
ph (y ∅)

-- this is bad...
theorem df_sb {ph : setvar → wff} {y : setvar → setvar} (x : setvar) :
  wsb ph y ↔ (x ≡ y x → ph x) ∧ ∃ x : setvar, x ≡ y x ∧ ph x :=
sorry

-- Hm, this is false
theorem ax_6 {y : setvar → setvar} : ¬ ∀ x, ¬ x ≡ y x :=
sorry

theorem ax_7 {x y z : setvar} : wi (wceq (cv x) (cv y)) (wi (wceq (cv x) (cv z)) (wceq (cv y) (cv z))) :=
λ h1 h2, h1.symm.trans h2

def wcel : «class» → «class» → wff := (∈)
local notation x ` ∈ ` y := (↑x : «class») ∈ (↑y : «class»)
theorem ax_8 {x y z : setvar} (h : x ≡ y) (h' : x ∈ z) : y ∈ z := h ▸ h'

theorem ax_9 {x y z : setvar} (h : x ≡ y) (h' : z ∈ x) : z ∈ y := h ▸ h'

theorem ax_10 {ph : setvar → wff} (h : ¬ ∀ x, ph x) (x:setvar) : ¬ ∀ x, ph x := h

theorem ax_11 {ph : setvar → setvar → wff} (h : ∀ x y, ph x y) (y x) : ph x y := h x y

theorem ax_11_b {ph : setvar → wff} (h : ∀ x x : setvar, ph x) : ∀ x x : setvar, ph x := h

theorem ax_12 {ph : setvar → setvar → wff} (x y) (h : x ≡ y) (h2 : ∀ y, ph x y) (x') (h3 : x' ≡ y) : ph x' y :=
weq'.1 (h.trans h3.symm) ▸ h2 y

theorem ax_12_b {ph : setvar → wff} (x : setvar) (h : x ≡ x) (h2 : ∀ x, ph x) (x') (h3 : x' ≡ x') : ph x' := h2 x'

-- false
theorem ax_13 {y z : setvar → setvar} (x) : ¬ x ≡ y x → y x ≡ z x → ∀ x, y x ≡ z x := sorry

end mm0