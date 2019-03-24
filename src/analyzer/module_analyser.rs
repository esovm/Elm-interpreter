use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;

use analyzer::dependency_sorter::sort_statements;
use analyzer::function_analyzer::analyze_function;
use analyzer::inter_mod_analyzer::ModuleInfo;
use analyzer::static_env::StaticEnv;
use analyzer::TypeError;
use ast::*;
use core::register_core;
use errors::*;
use errors::ElmError;
use errors::LoaderError;
use loader::Declaration;
use loader::LoadedModule;
use loader::ModuleLoader;
use source::SourceCode;
use types::*;
use util::build_fun_type;
use util::create_vec_inv;
use util::get_fun_return;
use util::qualified_name;
use util::visitors::type_visitor;

pub fn analyze_module_imports(modules: &HashMap<String, LoadedModule>, env: &mut StaticEnv, imports: &Vec<Import>) -> Result<(), ElmError> {
    for import in imports {
        let name = import.path.join(".");
        let module = modules.get(&name)
            .ok_or(ElmError::Loader { info: LoaderError::MissingImport { name } })?;

        match (&import.alias, &import.exposing) {
            (None, Some(me)) => {
                let decls = match me {
                    ModuleExposing::Just(exp) => {
                        get_exposed_decls(&module.declarations, exp)
                            .map_err(|e| ElmError::Interpreter { info: e })?
                    },
                    ModuleExposing::All => {
                        module.declarations.clone()
                    },
                };

                for decl in &decls {
                    match decl {
                        Declaration::Def(name, ty) => env.add_definition(name, ty.clone()),
                        Declaration::Alias(name, ty) => env.add_alias(name, ty.clone()),
                        Declaration::Adt(name, adt) => env.add_adt(name, adt.clone()),
                    }
                }
            },
            (Some(it), None) => {
                // TODO
                unimplemented!()
            },
            _ => {
                panic!("Invalid combination of alias and exposing for import: {:?}", import)
            }
        }
    }
    Ok(())
}

/* The environment must contain all the imports resolved */
pub fn analyze_module_declarations(env: &mut StaticEnv, statements: &Vec<Statement>) -> Result<Vec<Declaration>, TypeError> {
    let statements = sort_statements(statements)
        .map_err(|e| {
            TypeError::CyclicStatementDependency(e)
        })?;

    let mut declarations = vec![];
    let mut errors = vec![];

    for stm in statements {
        match analyze_statement(env, stm) {
            Ok(decls) => {
                for decl in decls.into_iter() {
                    declarations.push(decl.clone());
                    match decl {
                        Declaration::Def(name, ty) => {
                            env.add_definition(&name, ty);
                        }
                        Declaration::Alias(name, ty) => {
                            env.add_alias(&name, ty);
                        }
                        Declaration::Adt(name, adt) => {
                            env.add_adt(&name, adt);
                        }
                    }
                }
            }
            Err(e) => {
                // TODO add parameter to exit on first error
//                return Err(e);
                errors.push(e);
            }
        }
    }

    if errors.is_empty() {
        Ok(declarations)
    } else {
        Err(TypeError::List(errors))
    }
}

fn get_default_header() -> ModuleHeader {
    ModuleHeader { name: "Main".to_owned(), exposing: ModuleExposing::All }
}

fn analyze_statement(env: &mut StaticEnv, stm: &Statement) -> Result<Vec<Declaration>, TypeError> {
    let decls = match stm {
        Statement::Alias(name, vars, ty) => {
            println!("analyze_type_alias: {}", name);
            analyze_type_alias(name, vars, ty)?
        }
        Statement::Adt(name, vars, variants) => {
            println!("analyze_adt: {}", name);
            analyze_adt(name, vars, variants)?
        }
        Statement::Port(name, ty) => {
            println!("analyze_port: {}", name);
            analyze_port(name, ty)?
        }
        Statement::Def(def) => {
            vec![Declaration::Def(def.name.clone(), analyze_function(env, def)?)]
        }
        Statement::Infix(_, _, name, def) => {
            println!("ignore infix operator: {}", name);

            match env.find_definition(name) {
                None => {
                    let func = env.find_definition(def);
                    match func {
                        Some(ty) => {
                            vec![Declaration::Def(name.clone(), ty)]
                        }
                        _ => vec![]
                    }
                }
                _ => vec![]
            }
        }
    };

    Ok(decls)
}

fn analyze_port(name: &str, ty: &Type) -> Result<Vec<Declaration>, TypeError> {
    Ok(vec![
        Declaration::Def(name.to_owned(), ty.clone())
    ])
}

fn analyze_adt(name: &str, decl_vars: &Vec<String>, variants: &Vec<(String, Vec<Type>)>) -> Result<Vec<Declaration>, TypeError> {
    let vars: Vec<Type> = decl_vars.iter()
        .map(|v| Type::Var(v.to_owned()))
        .collect();

    let adt_variants = variants.iter()
        .map(|(name, types)| {
            AdtVariant {
                name: name.clone(),
                types: types.clone(),
            }
        })
        .collect();

    let adt = Arc::new(Adt {
        name: name.to_owned(),
        types: decl_vars.clone(),
        variants: adt_variants,
    });

    let adt_type = Type::Tag(name.to_owned(), vars);
    let mut decls = vec![Declaration::Adt(name.to_owned(), adt.clone())];

    for (variant_name, params) in variants {
        let variant_type = build_fun_type(&create_vec_inv(params, adt_type.clone()));

        decls.push(Declaration::Def(variant_name.clone(), variant_type));
    }

    Ok(decls)
}

fn analyze_type_alias(name: &str, decl_vars: &Vec<String>, ty: &Type) -> Result<Vec<Declaration>, TypeError> {
    let mut used_vars: HashSet<String> = HashSet::new();

    type_visitor(&mut used_vars, ty, &|set, node| {
        if let Type::Var(var) = &node {
            set.insert(var.clone());
        }
    });

    if used_vars.len() < decl_vars.len() {
        let unused_vars = decl_vars.into_iter()
            .filter(|t| !used_vars.contains(*t))
            .map(|t| t.clone())
            .collect::<Vec<String>>();

        return Err(TypeError::UnusedTypeVariables(unused_vars));
    }

    if used_vars.len() > decl_vars.len() {
        let unknown_vars = used_vars.into_iter()
            .filter(|t| !decl_vars.contains(t))
            .map(|t| t.clone())
            .collect::<Vec<String>>();

        return Err(TypeError::UndeclaredTypeVariables(unknown_vars));
    }


    let mut decls: Vec<Declaration> = vec![
        Declaration::Alias(name.to_owned(), ty.clone())
    ];

    // If the type alias is for an record, a auxiliary constructor function is created
    if let Type::Record(entries) = ty {
        let mut args: Vec<Type> = entries.iter()
            .map(|(_, ty)| ty.clone())
            .collect();

        args.push(ty.clone());

        decls.push(Declaration::Def(name.to_owned(), build_fun_type(&args)))
    }

    Ok(decls)
}

fn get_exposed_decls(all_decls: &Vec<Declaration>, exposed: &Vec<Exposing>) -> Result<Vec<Declaration>, RuntimeError> {
    let mut exposed_decls = Vec::new();

    for exp in exposed.iter() {
        match exp {
            Exposing::Adt(name, adt_exp) => {
                match adt_exp {
                    AdtExposing::Variants(variants) => {
                        for it in all_decls.iter() {
                            if let Declaration::Def(variant_name, ty) = it {
                                if variants.contains(variant_name) {
                                    if let Type::Tag(tag_name, _) = get_fun_return(ty) {
                                        if &tag_name == name {
                                            exposed_decls.push(it.clone());
                                        }
                                    }
                                }
                            }
                        }
                    }
                    AdtExposing::All => {
                        for it in all_decls.iter() {
                            if let Declaration::Def(_, ty) = it {
                                if let Type::Tag(tag_name, _) = get_fun_return(ty) {
                                    if &tag_name == name {
                                        exposed_decls.push(it.clone());
                                    }
                                }
                            }
                        }
                    }
                }

                let decl = all_decls.iter()
                    .find(|decl| {
                        if let Declaration::Adt(adt_name, _) = decl {
                            adt_name == name
                        } else {
                            false
                        }
                    })
                    .map(|decl| decl.clone())
                    .ok_or_else(|| RuntimeError::MissingExposing(name.clone(), all_decls.clone()))?;

                exposed_decls.push(decl);
            }
            Exposing::Type(name) => {
                let decl = all_decls.iter()
                    .find(|decl| {
                        if let Declaration::Alias(alias_name, _) = decl {
                            alias_name == name
                        } else if let Declaration::Adt(adt_name, _) = decl {
                            adt_name == name
                        } else {
                            false
                        }
                    })
                    .map(|decl| decl.clone())
                    .ok_or_else(|| RuntimeError::MissingExposing(name.clone(), all_decls.clone()))?;

                exposed_decls.push(decl);
            }
            Exposing::BinaryOperator(name) => {
                let decl = all_decls.iter()
                    .find(|decl| {
                        if let Declaration::Def(def_name, _) = decl {
                            def_name == name
                        } else {
                            false
                        }
                    })
                    .map(|decl| decl.clone());

                if let Some(decl) = decl {
                    exposed_decls.push(decl);
                }
            }
            Exposing::Definition(name) => {
                let decl = all_decls.iter()
                    .find(|decl| {
                        if let Declaration::Def(def_name, _) = decl {
                            def_name == name
                        } else {
                            false
                        }
                    })
                    .map(|decl| decl.clone())
                    .ok_or_else(|| RuntimeError::MissingExposing(name.clone(), all_decls.clone()))?;

                exposed_decls.push(decl);
            }
        }
    }

    Ok(exposed_decls)
}


#[cfg(test)]
mod tests {
    use util::StringConversion;

    use super::*;

    #[test]
    fn check_type_alias_base() {
        let ty = Type::Unit;
        assert_eq!(
            analyze_type_alias("A", &vec![], &ty),
            Ok(vec![Declaration::Alias("A".s(), ty)])
        );
    }

    #[test]
    fn check_type_alias_1_var() {
        let ty = Type::Var("a".s());
        assert_eq!(
            analyze_type_alias("A", &vec!["a".s()], &ty),
            Ok(vec![Declaration::Alias("A".s(), ty)])
        );
    }

    #[test]
    fn check_type_alias_missing_var() {
        let ty = Type::Var("a".s());
        assert_eq!(
            analyze_type_alias("A", &vec![], &ty),
            Err(TypeError::UndeclaredTypeVariables(vec!["a".s()]))
        );
    }

    #[test]
    fn check_type_alias_extra_var() {
        let ty = Type::Var("a".s());
        assert_eq!(
            analyze_type_alias("A", &vec!["a".s(), "b".s()], &ty),
            Err(TypeError::UnusedTypeVariables(vec!["b".s()]))
        );
    }
}
