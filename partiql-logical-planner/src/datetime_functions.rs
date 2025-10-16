use partiql_catalog::call_defs::{CallDef, CallSpec, CallSpecArg};
use partiql_logical as logical;

pub fn function_call_def_current_time() -> CallDef {
    CallDef {
        names: vec!["current_time"],
        overloads: vec![CallSpec {
            input: vec![],
            output: Box::new(|args| {
                logical::ValueExpr::Call(logical::CallExpr {
                    name: logical::CallName::CurrentTime,
                    arguments: args,
                })
            }),
        }],
    }
}

pub fn function_call_def_current_timestamp() -> CallDef {
    CallDef {
        names: vec!["current_timestamp"],
        overloads: vec![CallSpec {
            input: vec![],
            output: Box::new(|args| {
                logical::ValueExpr::Call(logical::CallExpr {
                    name: logical::CallName::CurrentTimestamp,
                    arguments: args,
                })
            }),
        }],
    }
}

pub(crate) fn function_call_def_to_string() -> CallDef {
    CallDef {
        names: vec!["to_string"],
        overloads: vec![CallSpec {
            input: vec![CallSpecArg::Positional, CallSpecArg::Positional],
            output: Box::new(|args| {
                logical::ValueExpr::Call(logical::CallExpr {
                    name: logical::CallName::ToString,
                    arguments: args,
                })
            }),
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_time_registration() {
        let def = function_call_def_current_time();
        assert_eq!(def.names, vec!["current_time"]);
        assert_eq!(def.overloads.len(), 1);
        assert_eq!(def.overloads[0].input.len(), 0);
    }

    #[test]
    fn test_current_timestamp_registration() {
        let def = function_call_def_current_timestamp();
        assert_eq!(def.names, vec!["current_timestamp"]);
        assert_eq!(def.overloads.len(), 1);
        assert_eq!(def.overloads[0].input.len(), 0);
    }
}
