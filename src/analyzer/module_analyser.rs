use std::collections::HashSet;
use std::sync::Arc;

use analyzer::dependency_sorter::sort_statement_dependencies;
use analyzer::function_analyzer::analyze_function;
use analyzer::inter_mod_analyzer::Declaration;
use analyzer::inter_mod_analyzer::Declarations;
use analyzer::inter_mod_analyzer::InterModuleInfo;
use analyzer::inter_mod_analyzer::ModulePath;
use analyzer::static_env::StaticEnv;
use analyzer::TypeError;
use ast::*;
use types::*;
use util::build_fun_type;
use util::create_vec_inv;
use util::get_fun_return;
use util::qualified_name;
use util::visitors::type_visitor;

#[derive(Debug, PartialEq, Clone)]
pub struct CheckedModule {
    pub path: ModulePath,
    pub ast: Module,
    pub env: StaticEnv,
    pub exposing: Vec<Declaration>,
}

pub fn analyze_module(info: &InterModuleInfo, path: &ModulePath, module: Module) -> Result<CheckedModule, TypeError> {
    let header = module.header.clone().unwrap_or_else(get_default_header);

    let mut env = load_import_dependencies(info, &module)?;
    let all_decls = analyze_module_declarations(&mut env, &module)?;

    let exposing = match header.exposing {
        ModuleExposing::Just(exposed) => {
            get_exposed_decls(&all_decls, &exposed)?
        }
        ModuleExposing::All => {
            all_decls
        }
    };

    Ok(CheckedModule {
        path: path.clone(),
        ast: module,
        env,
        exposing,
    })
}

fn load_import_dependencies(info: &InterModuleInfo, module: &Module) -> Result<StaticEnv, TypeError> {
    let mut env = StaticEnv::new();

    for import in &module.imports {
        let module = info.get(&import.path)
            .ok_or_else(|| TypeError::MissingModule(import.path.clone()))?;

        if let Some(alias) = &import.alias {
            for decl in &module.exposing {
                match decl {
                    Declaration::Def(name, ty) => {
                        env.add_definition(&qualified_name(&[alias.clone()], name), ty.clone());
                    }
                    Declaration::Alias(name, ty) => {
                        env.add_alias(&qualified_name(&[alias.clone()], name), ty.clone());
                    }
                    Declaration::Adt(name, adt) => {
                        env.add_adt(&qualified_name(&[alias.clone()], name), adt.clone());
                    }
                }
            }
        }

        if let Some(exposing) = &import.exposing {
            let exposed = match exposing {
                ModuleExposing::Just(exposed) => {
                    get_exposed_decls(&module.exposing, exposed)?
                }
                ModuleExposing::All => {
                    module.exposing.clone()
                }
            };

            for decl in &exposed {
                match decl {
                    Declaration::Def(name, ty) => {
                        env.add_definition(name, ty.clone());
                    }
                    Declaration::Alias(name, ty) => {
                        env.add_alias(name, ty.clone());
                    }
                    Declaration::Adt(name, adt) => {
                        env.add_adt(name, adt.clone());
                    }
                }
            }
        }

        if import.exposing.is_none() && import.alias.is_none() {
            for decl in &module.exposing {
                match decl {
                    Declaration::Def(name, ty) => {
                        env.add_definition(&qualified_name(&import.path, name), ty.clone());
                    }
                    Declaration::Alias(name, ty) => {
                        env.add_alias(&qualified_name(&import.path, name), ty.clone());
                    }
                    Declaration::Adt(name, adt) => {
                        env.add_adt(&qualified_name(&import.path, name), adt.clone());
                    }
                }
            }
        }
    }

    Ok(env)
}

fn get_default_header() -> ModuleHeader {
    ModuleHeader { name: "Main".to_owned(), exposing: ModuleExposing::All }
}

/* The environment must contain all the imports resolved */
fn analyze_module_declarations(env: &mut StaticEnv, module: &Module) -> Result<Declarations, TypeError> {
    let statements = sort_statement_dependencies(&module.statements);
    let mut declarations = Declarations::new();

    for stm in statements {
        let decls = analyze_statement(env, stm)?;

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

    Ok(declarations)
}

fn analyze_statement(env: &mut StaticEnv, stm: &Statement) -> Result<Declarations, TypeError> {
    let decls = match stm {
        Statement::Alias(name, vars, ty) => {
            analyze_type_alias(name, vars, ty)?
        }
        Statement::Adt(name, vars, variants) => {
            analyze_adt(name, vars, variants)?
        }
        Statement::Port(name, ty) => {
            analyze_port(name, ty)?
        }
        Statement::Def(def) => {
            vec![Declaration::Def(def.name.clone(), analyze_function(env, def)?)]
        }
    };

    Ok(decls)
}

fn analyze_port(name: &str, ty: &Type) -> Result<Declarations, TypeError> {
    Ok(vec![
        Declaration::Def(name.to_owned(), ty.clone())
    ])
}

fn analyze_adt(name: &str, decl_vars: &Vec<String>, variants: &Vec<(String, Vec<Type>)>) -> Result<Declarations, TypeError> {
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

fn analyze_type_alias(name: &str, decl_vars: &Vec<String>, ty: &Type) -> Result<Declarations, TypeError> {
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


    let mut decls: Declarations = vec![
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

fn get_exposed_decls(all_decls: &Declarations, exposed: &Vec<Exposing>) -> Result<Declarations, TypeError> {
    let mut exposed_decls = Declarations::new();

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
                    .ok_or_else(|| TypeError::InternalError)?;

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
                    .ok_or_else(|| TypeError::InternalError)?;

                exposed_decls.push(decl);
            }
            Exposing::Definition(name) | Exposing::BinaryOperator(name) => {
                let decl = all_decls.iter()
                    .find(|decl| {
                        if let Declaration::Def(def_name, _) = decl {
                            def_name == name
                        } else {
                            false
                        }
                    })
                    .map(|decl| decl.clone())
                    .ok_or_else(|| TypeError::InternalError)?;

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
