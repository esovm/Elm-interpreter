use analyzer::type_check_function;
use interpreter::dynamic_env::DynamicEnv;
use interpreter::expression_eval::eval_expr;
use interpreter::RuntimeError;
use interpreter::RuntimeError::TODO;
use types::Expr;
use types::Fun;
use types::Statement;
use types::Type;
use types::Value;
use types::ValueDefinition;
use util::build_fun_type;
use util::create_vec_inv;
use util::StringConversion;

pub fn eval_statement(env: &mut DynamicEnv, stm: &Statement) -> Result<Option<Value>, RuntimeError> {
    match stm {
        Statement::Alias(name, _, ty) => {
            env.types.add(name, ty.clone());
        }
        Statement::Adt(name, vars, variants) => {
            let vars: Vec<Type> = vars.iter()
                .map(|v| Type::Var(v.to_owned()))
                .collect();

            let ty = Type::Tag(name.clone(), vars);

            env.types.add(name, ty.clone());

            for (var_name, params) in variants {
                let var_ty = build_fun_type(&create_vec_inv(params, ty.clone()));

                let value = if params.is_empty() {
                    Value::Adt(var_name.clone(), vec![], name.clone())
                } else {
                    let ty = Type::Fun(
                        Box::from(Type::Tag("String".s(), vec![])),
                        Box::from(Type::Fun(
                            Box::from(Type::Tag("String".s(), vec![])),
                            Box::from(var_ty.clone()),
                        )),
                    );

                    Value::Fun {
                        args: vec![Value::String(var_name.clone()), Value::String(name.clone())],
                        arg_count: (params.len() + 2) as u32,
                        fun: Fun::Builtin(7, ty),
                    }
                };

                env.add(var_name, value, var_ty);
            }
        }
        Statement::Port(_name, _ty) => {
            // TODO
        }
        Statement::Def(def) => {
            let def_ty = type_check_function(&mut env.types, def)
                .map_err(|e| TODO(format!("{:?}", e)))?;

            let ValueDefinition { name, patterns, expr } = &def.1;

            let value = Value::Fun {
                args: vec![],
                arg_count: patterns.len() as u32,
                fun: Fun::Expr(patterns.clone(), expr.clone(), def_ty.clone()),
            };

            env.add(name, value.clone(), def_ty);

            let ret = if patterns.len() == 0 { eval_expr(env, expr)? } else { value };
            return Ok(Some(ret));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use interpreter::expression_eval::get_value_type;
    use nom::*;
    use nom::verbose_errors::*;
    use parsers::from_code;
    use parsers::from_code_stm;
    use super::*;
    use tokenizer::tokenize;
    use types::Pattern;
    use types::Type;
    use util::builtin_fun_of;
    use util::StringConversion;

    fn formatted(env: &mut DynamicEnv, stm: &Statement) -> String {
        let result = eval_statement(env, stm);
        let option = result.unwrap();
        let value = option.unwrap();
        let ty = get_value_type(&value);

        format!("{} : {}", value, ty)
    }

    fn formatted_expr(env: &mut DynamicEnv, expr: &Expr) -> String {
        let result = eval_expr(env, expr);
        let value = result.unwrap();
        let ty = get_value_type(&value);

        format!("{} : {}", value, ty)
    }

    #[test]
    fn check_constant() {
        let stm = from_code_stm(b"x = 1");
        let mut env = DynamicEnv::new();

        assert_eq!(formatted(&mut env, &stm), "1 : number".s());
    }

    #[test]
    fn check_identity() {
        let stm = from_code_stm(b"id value = value");
        let mut env = DynamicEnv::new();

        assert_eq!(formatted(&mut env, &stm), "<function> : a -> a".s());
    }

//    #[test]
//    fn check_recursivity() {
//        let stm = from_code_stm(b"fib num = case num of \n 0 -> 0\n 1 -> 1\n _ -> fib (num - 1) + fib (num - 2)");
//        let mut env = DynamicEnv::default_lang_env();
//
//        assert_eq!(formatted(&mut env, &stm), "<function> : Int -> number".s());
//    }

    #[test]
    fn check_adt() {
        let decl = from_code_stm(b"type Adt = A | B");
        let mut env = DynamicEnv::default_lang_env();

        eval_statement(&mut env, &decl).unwrap();

        assert_eq!(formatted_expr(&mut env, &from_code(b"A")), "A : Adt".s());
        assert_eq!(formatted_expr(&mut env, &from_code(b"B")), "B : Adt".s());
    }

//    #[test]
//    fn check_adt2() {
//        let decl = from_code_stm(b"type Adt a = A a | B Int");
//        let mut env = DynamicEnv::default_lang_env();
//
//        eval_statement(&mut env, &decl).unwrap();
//
//        assert_eq!(formatted_expr(&mut env, &from_code(b"A")), "<function> : a -> Adt a".s());
//        assert_eq!(formatted_expr(&mut env, &from_code(b"B")), "<function> : Int -> Adt a".s());
//        assert_eq!(formatted_expr(&mut env, &from_code(b"A 1")), "A 1 : Adt number".s());
//        assert_eq!(formatted_expr(&mut env, &from_code(b"B 1")), "B 1 : Adt a".s());
//    }
}