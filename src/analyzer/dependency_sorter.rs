use ast::Statement;
use analyzer::static_env::StaticEnv;
use ast::Pattern;
use analyzer::function_analyzer::analyze_function_arguments;
use std::collections::HashMap;
use ast::Type;
use std::collections::HashSet;
use ast::Expr;
use util::visitors::expr_visitor_block;
use util::qualified_name;
use util::visitors::type_visitor;
use ast::LetDeclaration;


pub fn sort_statement_dependencies(stms: &[Statement]) -> Vec<&Statement> {
    let mut dependencies: HashMap<&str, (&Statement, Vec<String>)> = HashMap::new();
    let local_names: Vec<&str> = stms.iter()
        .map(|s| get_stm_name(s))
        .collect();

    for stm in stms {
        let deps: Vec<String> = get_stm_dependencies(stm)
            .into_iter()
            .filter(|i| local_names.contains(&i.as_str()))
            .collect();

        dependencies.insert(get_stm_name(stm), (stm, deps));
    }

    let mut res: Vec<&Statement> = Vec::new();

    while !dependencies.is_empty() {
        let leafs: Vec<(&str, &Statement)> = dependencies.iter()
            .filter(|(_, (_, deps))| deps.is_empty())
            .map(|(name, (stm, _))| (*name, *stm))
            .collect();

        if leafs.is_empty() {
            // Cycle detected, the handling is done when the first
            // statement is processed and invalid references are found

            let missing: Vec<_> = stms.iter()
                .filter(|it| !res.contains(it))
                .collect();

            for stm in missing {
                res.push(stm);
            }

            return res;
        }

        for (leaf_name, leaf_stm) in leafs {
            res.push(leaf_stm);
            dependencies.remove(leaf_name);


            for (_, (_, deps)) in dependencies.iter_mut() {
                let indexes: Vec<usize> = deps.iter()
                    .enumerate()
                    .filter(|(_, dep)| dep.as_str() == leaf_name)
                    .map(|(index, _)| index)
                    .collect();

                for index in indexes {
                    deps.remove(index);
                }
            }
        }
    }

    res
}

fn get_stm_dependencies(def: &Statement) -> Vec<String> {
    match def {
        Statement::Alias(_, _, ty) => { get_type_dependencies(ty) }
        Statement::Port(_, ty) => { get_type_dependencies(ty) }
        Statement::Def(def) => {
            let mut fake_env = StaticEnv::new();
            add_patterns(&mut fake_env, &def.patterns);

            get_expr_dependencies(&mut fake_env, &def.expr)
        }
        Statement::Adt(_, _, branches) =>
            branches.iter()
                .map(|(_, tys)| {
                    tys.iter().map(|ty| get_type_dependencies(ty)).flatten()
                })
                .flatten()
                .collect(),
        Statement::Infix(_, _, _, _) => vec![]
    }
}

fn add_patterns(env: &mut StaticEnv, patterns: &Vec<Pattern>) {
    for (_, entries) in analyze_function_arguments(env, patterns, &None) {
        for (name, _) in entries {
            env.add_definition(&name, Type::Unit);
        }
    }
}

fn get_type_dependencies(ty: &Type) -> Vec<String> {
    let mut refs: HashSet<String> = HashSet::new();

    type_visitor(&mut refs, ty, &|state, sub_ty| {
        match sub_ty {
            Type::Var(_) => {}
            Type::Tag(name, _) => { state.insert(name.clone()); }
            Type::Fun(_, _) => {}
            Type::Unit => {}
            Type::Tuple(_) => {}
            Type::Record(_) => {}
            Type::RecExt(name, _) => { state.insert(name.clone()); }
        }
    });

    refs.into_iter().collect()
}

fn get_expr_dependencies(env: &mut StaticEnv, expr: &Expr) -> Vec<String> {
    let mut local_refs: HashSet<String> = HashSet::new();

    expr_visitor_block(&mut (env, &mut local_refs), expr, &|(env, refs), sub_expr| {
        match sub_expr {
            Expr::RecordUpdate(name, _) => {
                if let None = env.find_definition(name) {
                    refs.insert(name.clone());
                }
            }
            Expr::QualifiedRef(path, name) => {
                let full_name = qualified_name(path, name);
                if let None = env.find_definition(&full_name) {
                    refs.insert(full_name);
                }
            }
            Expr::OpChain(_, ops) => {
                for op in ops {
                    if let None = env.find_definition(op) {
                        refs.insert(op.clone());
                    }
                }
            }
            Expr::Ref(name) => {
                if let None = env.find_definition(name) {
                    refs.insert(name.clone());
                }
            }

            Expr::RecordField(_, _) => {}
            Expr::RecordAccess(_) => {}
            Expr::If(_, _, _) => {}
            Expr::Case(_, _) => {}
            Expr::Application(_, _) => {}
            Expr::Literal(_) => {}

            Expr::Lambda(patterns, _) => {
                env.enter_block();
                add_patterns(env, patterns);
            }
            Expr::Let(decls, _) => {
                env.enter_block();
                for decl in decls {
                    match decl {
                        LetDeclaration::Def(def) => {
                            add_patterns(env, &def.patterns);
                        },
                        LetDeclaration::Pattern(pattern, _) => {
                            add_patterns(env, &vec![pattern.clone()]);
                        },
                    }
                }
            }
            _ => {}
        }
    }, &|(env, _), sub_expr| {
        match sub_expr {
            Expr::Lambda(_, _) => {
                env.exit_block();
            }
            Expr::Let(_, _) => {
                env.exit_block();
            }
            _ => {}
        }
    });

    local_refs.into_iter().collect()
}

fn get_stm_name(stm: &Statement) -> &str {
    match stm {
        Statement::Alias(name, _, _) => { name }
        Statement::Adt(name, _, _) => { name }
        Statement::Port(name, _) => { name }
        Statement::Def(def) => { &def.name }
        Statement::Infix(_, _, op, _) => { op }
    }
}

#[cfg(test)]
mod tests {
    use parsers::from_code_mod;
    use super::*;


    #[test]
    fn check_expr_dependencies() {
        let module = from_code_mod(b"\ny = x + 1\n\nx = 0\n\nz = y");
        let sorted = sort_statement_dependencies(&module.statements);

        let names: Vec<_> = sorted.iter().map(|stm| get_stm_name(stm)).collect();


        assert_eq!(names, vec!["x", "y", "z"]);
    }
}