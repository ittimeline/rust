use crate::transform::{MirPass, MirSource};
use rustc_middle::mir::*;
use rustc_middle::ty::TyCtxt;

pub struct MatchBranchSimplification;

// What's the intent of this pass?
// If one block is found that switches between blocks which both go to the same place
// AND both of these blocks set a similar const in their ->
// condense into 1 block based on discriminant AND goto the destination afterwards

impl<'tcx> MirPass<'tcx> for MatchBranchSimplification {
    fn run_pass(&self, tcx: TyCtxt<'tcx>, src: MirSource<'tcx>, body: &mut Body<'tcx>) {
        let param_env = tcx.param_env(src.def_id());
        let bbs = body.basic_blocks_mut();
        'outer: for bb_idx in bbs.indices() {
            let (discr, val, switch_ty, first, second) = match bbs[bb_idx].terminator().kind {
                TerminatorKind::SwitchInt {
                    discr: Operand::Move(ref place),
                    switch_ty,
                    ref targets,
                    ref values,
                    ..
                } if targets.len() == 2 && values.len() == 1 => {
                    (place, values[0], switch_ty, targets[0], targets[1])
                }
                // Only optimize switch int statements
                _ => continue,
            };

            // Check that destinations are identical, and if not, then don't optimize this block
            if &bbs[first].terminator().kind != &bbs[second].terminator().kind {
                continue;
            }

            // Check that blocks are assignments of consts to the same place or same statement,
            // and match up 1-1, if not don't optimize this block.
            let first_stmts = &bbs[first].statements;
            let scnd_stmts = &bbs[second].statements;
            if first_stmts.len() != scnd_stmts.len() {
                continue;
            }
            for (f, s) in first_stmts.iter().zip(scnd_stmts.iter()) {
                match (&f.kind, &s.kind) {
                    // If two statements are exactly the same just ignore them.
                    (f_s, s_s) if f_s == s_s => (),

                    (
                        StatementKind::Assign(box (lhs_f, Rvalue::Use(Operand::Constant(f_c)))),
                        StatementKind::Assign(box (lhs_s, Rvalue::Use(Operand::Constant(s_c)))),
                    ) if lhs_f == lhs_s => {
                        if let Some(f_c) = f_c.literal.try_eval_bool(tcx, param_env) {
                            // This should also be a bool because it's writing to the same place
                            let s_c = s_c.literal.try_eval_bool(tcx, param_env).unwrap();
                            // Check that only const assignments of opposite bool values are
                            // permitted.
                            if f_c != s_c {
                              continue
                            }
                        }
                        continue 'outer;
                    }
                    // If there are not exclusively assignments, then ignore this
                    _ => continue 'outer,
                }
            }
            // Take owenership of items now that we know we can optimize.
            let discr = discr.clone();

            bbs[bb_idx].terminator_mut().kind = TerminatorKind::Goto { target: first };
            for s in bbs[first].statements.iter_mut() {
                if let StatementKind::Assign(box (_, ref mut rhs)) = s.kind {
                    let size = tcx.layout_of(param_env.and(switch_ty)).unwrap().size;
                    let const_cmp = Operand::const_from_scalar(
                        tcx,
                        switch_ty,
                        crate::interpret::Scalar::from_uint(val, size),
                        rustc_span::DUMMY_SP,
                    );
                    *rhs = Rvalue::BinaryOp(BinOp::Eq, Operand::Move(discr), const_cmp);
                }
            }
        }
    }
}
