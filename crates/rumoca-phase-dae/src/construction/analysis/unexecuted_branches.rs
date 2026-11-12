//! Statement checking for the branches MLS §11.5 does not execute.
//!
//! When a value-proven specialization settles a function conditional's
//! conditions, `plan_function_conditional` plans only the selected branch and
//! the DAE lowering never sees the others. MLS §11.5 justifies that for
//! *execution* — but well-formedness is not execution. This module carries the
//! part of the base compiler's strength that the fold would otherwise drop.

use super::*;

/// Prove every unexecuted branch is still a legal algorithm section.
///
/// # Acceptance contract (SPEC_0008 §"Acceptance Contract Before Rejection")
///
/// MLS §11.2.1 makes an assignment statement's compatibility between its
/// right-hand-side expression and its component reference a property of *the
/// statement*. MLS §11.5 says which statements run; it does not say which are
/// well formed. Before this phase folded proven conditionals, an unexecuted
/// branch was lowered like any other and the DAE constructor rejected a
/// mismatched assignment there (`ED020`, "expression shape mismatch"). Folding
/// removes the lowering, so the check moves here and the strength is kept.
///
/// **Rejected.** Rank mismatches and fixed-extent mismatches, even in dead
/// arms; every proven mismatch on a reachable or unsettled path. Both scopes
/// use the same `expression_shape` rules as executed statements.
///
/// **Exempt — not an error, and deliberately so.** Anything this scope cannot
/// answer *without owning a proof the executed path never needs*:
///
/// * a value containing a function call. Specializing the callee is what would
///   answer its result shape, and an unexecuted branch must mint no
///   certificate: `symmetricOrientation(3)`'s unexecuted arm calls
///   `symmetricOrientation(1)`, which the program never evaluates.
/// * an extent only the unexecuted path settles — a comprehension, a compact
///   range, or a subscript over a value this specialization does not fix. MLS
///   §4.4.2 evaluability is required of the extents a *constructed* function
///   declares, and no function is constructed from this branch.
/// * every statement form other than an assignment, plus the nested bodies of
///   loops and conditionals, which are walked for their assignments only.
/// * a value-dependent *extent* mismatch in a proven unexecuted arm, which
///   cannot be established without specializing the inputs. Its extents may
///   read the values whose specialization makes the arm unreachable: MSL
///   `symmetricOrientationMatrix(3)` keeps
///   `oM[1:mBase, 1:mBase] := {o[1:mBase], -o[1:mBase]}` under
///   `if mBase == 2`, a [2, 3] value for a [3, 3] target only because
///   `mBase = 3`. In the generic function both extents are run-time sizes
///   (MLS §10.6), so the mismatch is the run-time check of an arm that never
///   runs, not an ill-formed statement. A *rank* mismatch does not depend on
///   any value and is still rejected there.
///
/// Fixed extents are independently checked without specialization input values;
/// an unknown guard never grants the value-dependent extent exemption.
///
/// **Owner.** Function-source normalization and proven-branch planning.
///
/// **Evidence.** `rumoca/tests/function_proven_branch_test.rs`:
/// `a_dead_arm_shape_error_is_still_rejected` (rejected) and
/// `a_dead_arm_call_no_specialization_can_prove_stays_accepted` (exempt).
pub(super) fn check_unexecuted_branches(
    blocks: &[rumoca_core::StatementBlock],
    fallback: Option<&[rumoca_core::Statement]>,
    selected: Option<usize>,
    context: FunctionValidationContext<'_>,
) -> Result<(), ToDaeError> {
    let fixed_shapes = function_shapes::call_free_fixed_function_shapes(context.function);
    for (ordinal, block) in blocks.iter().enumerate() {
        if selected == Some(ordinal) {
            continue;
        }
        check_statements(
            &block.stmts,
            context,
            context.shapes,
            &fixed_shapes,
            Extents::Unexecuted,
        )?;
    }
    if selected.is_some()
        && let Some(fallback) = fallback
    {
        check_statements(
            fallback,
            context,
            context.shapes,
            &fixed_shapes,
            Extents::Unexecuted,
        )?;
    }
    Ok(())
}

/// Check assignment compatibility before semantics-preserving normalization.
///
/// Loop and scratch compaction may remove a source statement whose value is
/// dead, but MLS §11.2.1 well-formedness is a property of that statement, not
/// of its liveness. Running this call-free proof before normalization retains
/// every mismatch the specialization can establish without minting a callee
/// certificate for an unexecuted path.
pub(super) fn check_function_assignment_shapes(
    statements: &[rumoca_core::Statement],
    context: FunctionValidationContext<'_>,
) -> Result<(), ToDaeError> {
    let fixed_shapes = function_shapes::call_free_fixed_function_shapes(context.function);
    check_statements(
        statements,
        context,
        context.shapes,
        &fixed_shapes,
        Extents::Checked,
    )
}

/// Only proven unexecuted paths may defer value-dependent extent comparisons.
#[derive(Clone, Copy)]
enum Extents {
    Checked,
    Unexecuted,
}

impl Extents {
    fn in_branch(self, unexecuted: bool) -> Self {
        if unexecuted { Self::Unexecuted } else { self }
    }
}

fn check_statements(
    statements: &[rumoca_core::Statement],
    context: FunctionValidationContext<'_>,
    shapes: &ShapeEnvironment,
    fixed_shapes: &ShapeEnvironment,
    extents: Extents,
) -> Result<(), ToDaeError> {
    for statement in statements {
        check_statement(statement, context, shapes, fixed_shapes, extents)?;
    }
    Ok(())
}

fn check_statement(
    statement: &rumoca_core::Statement,
    context: FunctionValidationContext<'_>,
    shapes: &ShapeEnvironment,
    fixed_shapes: &ShapeEnvironment,
    extents: Extents,
) -> Result<(), ToDaeError> {
    match statement {
        rumoca_core::Statement::Assignment { comp, value, span } => {
            check_assignment(comp, value, *span, context, shapes, fixed_shapes, extents)
        }
        rumoca_core::Statement::For {
            indices, equations, ..
        } => {
            // MLS §11.2.2 opens each index as a fresh scalar of the loop, so the
            // body is checked in a scope where the index shadows any enclosing
            // coordinate and carries no proven value.
            let mut loop_shapes = shapes.clone();
            let mut fixed_loop_shapes = fixed_shapes.clone();
            for index in indices {
                loop_shapes.insert(VarName::new(&index.ident), Vec::new());
                fixed_loop_shapes.insert(VarName::new(&index.ident), Vec::new());
            }
            check_statements(
                equations,
                context,
                &loop_shapes,
                &fixed_loop_shapes,
                extents,
            )
        }
        rumoca_core::Statement::While { block, .. } => {
            let unexecuted = matches!(
                shapes.proven_value(&block.cond),
                Some(ProvenValue::Boolean(false))
            );
            check_statements(
                &block.stmts,
                context,
                shapes,
                fixed_shapes,
                extents.in_branch(unexecuted),
            )
        }
        rumoca_core::Statement::If {
            cond_blocks,
            else_block,
            ..
        } => check_conditional(
            cond_blocks,
            else_block.as_deref(),
            context,
            shapes,
            fixed_shapes,
            extents,
        ),
        rumoca_core::Statement::When { blocks, .. } => {
            for block in blocks {
                check_statements(&block.stmts, context, shapes, fixed_shapes, extents)?;
            }
            Ok(())
        }
        rumoca_core::Statement::FunctionCall { .. }
        | rumoca_core::Statement::Reinit { .. }
        | rumoca_core::Statement::Assert { .. }
        | rumoca_core::Statement::Empty { .. }
        | rumoca_core::Statement::Return { .. }
        | rumoca_core::Statement::Break { .. } => Ok(()),
    }
}

fn check_conditional(
    blocks: &[rumoca_core::StatementBlock],
    fallback: Option<&[rumoca_core::Statement]>,
    context: FunctionValidationContext<'_>,
    shapes: &ShapeEnvironment,
    fixed_shapes: &ShapeEnvironment,
    extents: Extents,
) -> Result<(), ToDaeError> {
    // A false arm is dead even when a later guard is unknown. An unknown arm
    // remains reachable even when a later guard is true; that true guard only
    // excludes the statements after its own arm in the ordered scan.
    let mut remaining = true;
    for block in blocks {
        let condition = shapes.proven_value(&block.cond);
        let unexecuted = !remaining || matches!(condition, Some(ProvenValue::Boolean(false)));
        check_statements(
            &block.stmts,
            context,
            shapes,
            fixed_shapes,
            extents.in_branch(unexecuted),
        )?;
        if matches!(condition, Some(ProvenValue::Boolean(true))) {
            remaining = false;
        }
    }
    check_statements(
        fallback.unwrap_or_default(),
        context,
        shapes,
        fixed_shapes,
        extents.in_branch(!remaining),
    )
}

fn check_assignment(
    component: &rumoca_core::ComponentReference,
    value: &Expression,
    span: Span,
    context: FunctionValidationContext<'_>,
    shapes: &ShapeEnvironment,
    fixed_shapes: &ShapeEnvironment,
    extents: Extents,
) -> Result<(), ToDaeError> {
    let (Some(target), Some(found)) = (
        call_free_target_shape(component, shapes),
        call_free_expression_shape(value, shapes),
    ) else {
        return Ok(());
    };
    // MLS §10.6.13 lets a scalar right-hand side fill an array target, which is
    // the one shape relation an assignment admits beyond equality.
    if target == found || found.is_empty() {
        return Ok(());
    }
    if matches!(extents, Extents::Unexecuted) && target.len() == found.len() {
        match (
            call_free_target_shape(component, fixed_shapes),
            call_free_expression_shape(value, fixed_shapes),
        ) {
            (Some(target), Some(found)) if target != found => {}
            _ => return Ok(()),
        }
    }
    Err(ToDaeError::unsupported_flat(
        "function conditional",
        format!(
            "`{}` assigns a value of shape {found:?} to a target of shape {target:?}; MLS \
             §11.2.1 requires compatible assignments, including overwritten statements \
             and fixed-extent assignments in unreachable branches",
            context.function.name
        ),
        span,
    ))
}
