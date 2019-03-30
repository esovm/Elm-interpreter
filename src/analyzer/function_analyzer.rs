use analyzer::Analyser;
use analyzer::expression_analyzer::analyze_expression;
use analyzer::pattern_analyzer::*;
use analyzer::PatternMatchingError;
use analyzer::static_env::StaticEnv;
use analyzer::type_helper::is_assignable;
use analyzer::TypeError;
use ast::*;
use typed_ast::expr_type;
use util::build_fun_type;
use util::create_vec_inv;
use util::StringConversion;

pub fn analyze_function(env: &mut StaticEnv, fun: &Definition) -> Result<Type, TypeError> {
    let expr = Analyser::from(env.clone()).analyse_definition(fun)?;
    Ok(expr.header)
}

#[cfg(test)]
mod tests {
    use ast::Statement;
    use constructors::*;
    use core::register_core;
    use test_utils::Test;

    use super::*;

    fn from_code_def(code: &str) -> Definition {
        let stm = Test::statement(code);
        match stm {
            Statement::Def(def) => def,
            _ => panic!("Expected definition but found: {:?}", stm)
        }
    }

    fn format_type(env: &mut StaticEnv, def: &Definition) -> String {
        format!("{}", analyze_function(env, def).expect("Run into type error"))
    }

    #[test]
    fn check_constant() {
        let def = from_code_def("const = 1");
        let mut env = StaticEnv::new();

        assert_eq!(format_type(&mut env, &def), "number");
    }

    #[test]
    fn check_identity() {
        let def = from_code_def("id arg1 = arg1");
        let mut env = StaticEnv::new();

        assert_eq!(format_type(&mut env, &def), "a -> a");
    }

    #[test]
    fn check_var_to_number() {
        let def = from_code_def("sum arg1 arg2 = arg1 + arg2");
        let mut env = StaticEnv::new();

        env.add_definition("+", build_fun_type(&vec![
            Type::Var("number".s()), Type::Var("number".s()), Type::Var("number".s())
        ]));

        assert_eq!(format_type(&mut env, &def), "number -> number -> number");
    }

    #[test]
    fn check_number_to_float() {
        let def = from_code_def("sum arg1 = arg1 + 1.5");
        let mut env = StaticEnv::new();

        env.add_definition("+", build_fun_type(&vec![
            Type::Var("number".s()), Type::Var("number".s()), Type::Var("number".s())
        ]));

        assert_eq!(format_type(&mut env, &def), "Float -> Float");
    }

    #[test]
    fn check_from_number_to_float() {
        let def = from_code_def("sum = (+) 1.5");
        let mut env = StaticEnv::new();

        env.add_definition("+", build_fun_type(&vec![
            Type::Var("number".s()), Type::Var("number".s()), Type::Var("number".s())
        ]));

        assert_eq!(format_type(&mut env, &def), "Float -> Float");
    }

    #[test]
    fn check_list_coercion() {
        let def = from_code_def("my = [1, 1.5]");
        let mut env = StaticEnv::new();

        assert_eq!(format_type(&mut env, &def), "List Float");
    }

    #[test]
    fn check_list_coercion2() {
        let def = from_code_def("my b = [1, 1.5, b]");
        let mut env = StaticEnv::new();

        assert_eq!(format_type(&mut env, &def), "Float -> List Float");
    }

    #[test]
    fn check_variable_separation() {
        let def = from_code_def("my a b = [a, b]");
        let mut env = StaticEnv::new();

        assert_eq!(format_type(&mut env, &def), "a -> a -> List a");
    }

    #[test]
    fn check_variable_separation2() {
        let def = from_code_def("my = (func, func)");
        let mut env = StaticEnv::new();

        env.add_definition("func", Type::Fun(
            Box::from(Type::Var("a".s())),
            Box::from(Type::Var("a".s())),
        ));

        assert_eq!(format_type(&mut env, &def), "( a -> a, b -> b )");
    }

    #[test]
    fn analyze_patterns_1() {
        analyze_pattern_test(
            type_int(),
            pattern_var("a"),
            "Int",
            r#"[("a", Tag("Int", []))]"#,
        );
    }

    #[test]
    #[ignore]
    fn analyze_patterns_2() {
        analyze_pattern_test(
            type_tag_args("Maybe", vec![type_var("item")]),
            pattern_tag_args("Just", vec![pattern_var("a")]),
            "Maybe item",
            r#"[("a", Var("item"))]"#,
        );
    }

    #[test]
    fn analyze_patterns_3() {
        analyze_pattern_test(
            type_int(),
            pattern_wildcard(),
            "Int",
            r#"[]"#,
        );
    }

    #[test]
    fn analyze_patterns_4() {
        analyze_pattern_test(
            type_unit(),
            pattern_unit(),
            "()",
            r#"[]"#,
        );
    }

    #[test]
    fn analyze_patterns_5() {
        analyze_pattern_test(
            type_tuple(vec![type_int(), type_unit()]),
            pattern_tuple(vec![pattern_var("a"), pattern_unit()]),
            "( Int, () )",
            r#"[("a", Tag("Int", []))]"#,
        );
    }

    #[test]
    fn analyze_patterns_6() {
        analyze_pattern_test(
            type_list(type_int()),
            pattern_list(vec![pattern_var("a"), pattern_var("b")]),
            "List Int",
            r#"[("a", Tag("Int", [])), ("b", Tag("Int", []))]"#,
        );
    }

    #[test]
    fn analyze_patterns_7() {
        analyze_pattern_test(
            type_record(vec![("x", type_int())]),
            pattern_record(vec!["x"]),
            "{ x : Int }",
            r#"[("x", Tag("Int", []))]"#,
        );
    }

    #[test]
    fn analyze_patterns_8() {
        analyze_pattern_test(
            type_list(type_int()),
            pattern_cons(pattern_var("x"), pattern_var("xs")),
            "List Int",
            r#"[("x", Tag("Int", [])), ("xs", Tag("List", [Tag("Int", [])]))]"#,
        );
    }

    #[test]
    fn analyze_patterns_9() {
        analyze_pattern_test(
            type_int(),
            pattern_int(1),
            "Int",
            r#"[]"#,
        );
    }

    #[test]
    fn analyze_patterns_10() {
        analyze_pattern_test(
            type_int(),
            pattern_alias(pattern_int(1), "x"),
            "Int",
            r#"[("x", Tag("Int", []))]"#,
        );
    }

    fn analyze_pattern_test(ty: Type, pattern: Pattern, type_str: &str, vars_str: &str) {
        let mut env = StaticEnv::new();
        register_core(&mut env);

        let (res_ty, vars) = analyze_pattern_with_type(&mut env, &pattern, ty)
            .expect("Error");

        assert_eq!(format!("{}", res_ty), type_str);
        assert_eq!(format!("{:?}", vars), vars_str);
    }
}