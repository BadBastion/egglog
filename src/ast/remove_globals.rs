//! Remove global variables from the program by translating
//! them into functions with no arguments.
//! This requires type information, so it is done after type checking.

use crate::{
    core::ResolvedCall, typechecking::FuncType, GenericAction, GenericActions, GenericExpr,
    GenericNCommand, ResolvedAction, ResolvedExpr, ResolvedFunctionDecl, ResolvedNCommand, Schema,
    TypeInfo,
};

/// Removes all globals from a program.
/// No top level lets are allowed after this pass,
/// nor any variable that references a global.
/// Adds new functions for global variables
/// and replaces references to globals with
/// references to the new functions.
/// e.g.
/// ```ignore
/// (let x 3)
/// (Add x x)
/// ```
/// becomes
/// ```ignore
/// (function x () i64)
/// (set (x) 3)
/// ```
pub(crate) fn remove_globals(
    type_info: &TypeInfo,
    prog: Vec<ResolvedNCommand>,
) -> Vec<ResolvedNCommand> {
    prog.into_iter()
        .map(|cmd| remove_globals_cmd(type_info, cmd))
        .flatten()
        .collect()
}

fn replace_global_var(type_info: &TypeInfo, expr: ResolvedExpr) -> ResolvedExpr {
    match expr {
        GenericExpr::Lit(ann, lit) => GenericExpr::Lit(ann, lit),
        GenericExpr::Var(ann, var) => GenericExpr::Call(
            (),
            ResolvedCall::Func(FuncType {
                name: var.name,
                input: vec![],
                output: var.sort.clone(),
                is_datatype: false,
                has_default: false,
            }),
            vec![],
        ),
        GenericExpr::Call(ann, head, args) => GenericExpr::Call(ann, head, args),
    }
}

fn remove_globals_expr(type_info: &TypeInfo, expr: ResolvedExpr) -> ResolvedExpr {
    expr.map(&mut |expr| replace_global_var(type_info, expr))
}

fn remove_globals_action(type_info: &TypeInfo, action: ResolvedAction) -> ResolvedAction {
    action.map_exprs(&mut |expr| replace_global_var(type_info, expr))
}

fn remove_globals_cmd(type_info: &TypeInfo, cmd: ResolvedNCommand) -> Vec<ResolvedNCommand> {
    match cmd {
        GenericNCommand::CoreAction(action) => match action {
            GenericAction::Let(ann, name, expr) => {
                let ty = expr.output_type(type_info);
                let func_decl = ResolvedFunctionDecl {
                    name: name.name,
                    schema: Schema {
                        input: vec![],
                        output: ty.name(),
                    },
                    default: None,
                    merge: None,
                    merge_action: GenericActions(vec![]),
                    cost: None,
                    unextractable: true,
                };
                vec![
                    GenericNCommand::Function(func_decl),
                    GenericNCommand::CoreAction(GenericAction::Set(
                        ann,
                        ResolvedCall::Func(FuncType {
                            name: name.name,
                            input: vec![],
                            output: ty,
                            is_datatype: false,
                            has_default: false,
                        }),
                        vec![],
                        remove_globals_expr(type_info, expr),
                    )),
                ]
            }
            _ => vec![GenericNCommand::CoreAction(remove_globals_action(
                type_info, action,
            ))],
        },
        _ => vec![cmd.map_exprs(&mut |expr| remove_globals_expr(type_info, expr))],
    }
}
