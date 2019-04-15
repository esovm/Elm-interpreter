use std::collections::HashMap;

use analyzer::Analyzer;
use analyzer::dependency_sorter::sort_statements;
use ast::AdtExposing;
use ast::Exposing;
use ast::Import;
use ast::ModuleExposing;
use ast::Statement;
use ast::Type;
use errors::ElmError;
use errors::InterpreterError;
use errors::LoaderError;
use errors::TypeError;
use errors::Wrappable;
use loader::AnalyzedModule;
use loader::Declaration;
use loader::declaration_name;
use loader::LoadedModule;
use loader::ModuleImport;
use typed_ast::TypedDefinition;

impl Analyzer {
    pub fn get_default_imports(&mut self, modules: &HashMap<String, AnalyzedModule>) -> Result<Vec<ModuleImport>, ElmError> {
        let mut module_imports = vec![];

        let basic = Import { path: vec!["Basics".to_string()], alias: None, exposing: Some(ModuleExposing::All) };
//        let list = Import { path: vec!["List".to_string()], alias: None, exposing: Some(ModuleExposing::Just(vec![Exposing::Adt("List".to_string(), AdtExposing::All)])) };
//        let maybe = Import { path: vec!["Maybe".to_string()], alias: None, exposing: Some(ModuleExposing::Just(vec![Exposing::Adt("Maybe".to_string(), AdtExposing::All)])) };
//        let result = Import { path: vec!["Result".to_string()], alias: None, exposing: Some(ModuleExposing::Just(vec![Exposing::Adt("Result".to_string(), AdtExposing::All)])) };
//        let string = Import { path: vec!["String".to_string()], alias: None, exposing: Some(ModuleExposing::Just(vec![Exposing::Type("String".to_string())])) };
//        let char_ = Import { path: vec!["Char".to_string()], alias: None, exposing: Some(ModuleExposing::Just(vec![Exposing::Type("Char".to_string())])) };
//        let tuple = Import { path: vec!["Tuple".to_string()], alias: None, exposing: None };
//        let debug = Import { path: vec!["Debug".to_string()], alias: None, exposing: None };

        self.analyze_import(modules, &mut module_imports, &basic)?;
//        self.analyze_import(modules, &mut module_imports, &list)?;
//        self.analyze_import(modules, &mut module_imports, &maybe)?;
//        self.analyze_import(modules, &mut module_imports, &result)?;
//        self.analyze_import(modules, &mut module_imports, &string)?;
//        self.analyze_import(modules, &mut module_imports, &char_)?;
//        self.analyze_import(modules, &mut module_imports, &tuple)?;
//        self.analyze_import(modules, &mut module_imports, &debug)?;

        Ok(module_imports)
    }
    pub fn analyze_module_imports(&mut self, modules: &HashMap<String, AnalyzedModule>, imports: &Vec<Import>) -> Result<Vec<ModuleImport>, ElmError> {
        let mut module_imports = vec![];

        for import in imports {
            self.analyze_import(modules, &mut module_imports, import)?;
        }

        Ok(module_imports)
    }

    fn import_qualified(&mut self, module_name: &str, alias: &str, module: &AnalyzedModule, result: &mut Vec<ModuleImport>) {
        for decl in &module.all_declarations {
            self.import_declaration(decl, module_name, alias, result);
        }
    }

    fn import_exposed(&mut self, module_name: &str, module: &AnalyzedModule, exposing: &ModuleExposing, result: &mut Vec<ModuleImport>) -> Result<(), ElmError> {
        let decls = match exposing {
            ModuleExposing::Just(exp) => {
                Self::get_exposed_decls(&module.all_declarations, exp).map_err(Wrappable::wrap)?
            }
            ModuleExposing::All => {
                module.all_declarations.clone()
            }
        };

        for decl in &module.all_declarations {
            self.import_declaration(decl, module_name, "", result);
        }

        Ok(())
    }

    fn import_declaration(&mut self, decl: &Declaration, module_name: &str, alias: &str, result: &mut Vec<ModuleImport>) {
        let aliased_name = if alias.is_empty() {
            declaration_name(decl).to_string()
        } else {
            format!("{}.{}", alias, declaration_name(decl))
        };

        match decl {
            Declaration::Port(name, ty) => {
                result.push(ModuleImport {
                    source: module_name.to_string(),
                    source_name: name.clone(),
                    destine_name: aliased_name.clone(),
                });
                self.env.add_definition(&aliased_name, ty.clone());

                result.push(ModuleImport {
                    source: module_name.to_string(),
                    source_name: name.clone(),
                    destine_name: name.clone(),
                });
                self.env.add_definition(name, ty.clone());
            }
            Declaration::Definition(name, def) => {
                result.push(ModuleImport {
                    source: module_name.to_string(),
                    source_name: name.clone(),
                    destine_name: aliased_name.clone(),
                });
                self.env.add_definition(&aliased_name, def.header.clone());

                result.push(ModuleImport {
                    source: module_name.to_string(),
                    source_name: name.clone(),
                    destine_name: name.clone(),
                });
                self.env.add_definition(name, def.header.clone());
            }
            Declaration::Alias(name, ty) => {
                self.env.add_alias(&aliased_name, ty.clone())
            }
            Declaration::Adt(name, adt) => {
                self.env.add_adt(&aliased_name, adt.clone())
            }
            Declaration::Infix(_, _) => {
                // Ignore
            }
        }
    }

    fn analyze_import(&mut self, modules: &HashMap<String, AnalyzedModule>, module_imports: &mut Vec<ModuleImport>, import: &Import) -> Result<(), ElmError> {
        let module_name = import.path.join(".");
        let module = modules.get(&module_name)
            .ok_or_else(|| LoaderError::MissingModule { module: module_name.clone() }.wrap())?;

        let alias = import.alias.as_ref().unwrap_or(&module_name);

        self.import_qualified(&module_name, alias, module, module_imports);

        if let Some(exposing) = &import.exposing {
            self.import_exposed(&module_name, module, exposing, module_imports)?;
        }

        Ok(())
    }

    pub fn analyze_module_declarations(&mut self, statements: &Vec<Statement>) -> Result<(Vec<Declaration>, Vec<TypedDefinition>), Vec<TypeError>> {
        let statements = sort_statements(statements)
            .map_err(|cycle| vec![TypeError::CyclicStatementDependency { cycle }])?;

        let mut declarations = vec![];
        let mut definitions = vec![];
        let mut errors = vec![];

        let mut internal_declarations = vec![];

        for stm in statements {
            match self.analyze_statement(stm) {
                Ok(decls) => {
                    internal_declarations.extend(decls.clone());

                    for decl in decls.into_iter() {
                        declarations.push(decl.clone());
                        match decl {
                            Declaration::Definition(name, def) => {
                                self.env.add_definition(&name, def.header.clone());
                                definitions.push(def);
                            }
                            Declaration::Port(name, ty) => {
                                self.env.add_definition(&name, ty);
                            }
                            Declaration::Alias(name, ty) => {
                                self.env.add_alias(&name, ty);
                            }
                            Declaration::Adt(name, adt) => {
                                self.env.add_adt(&name, adt);
                            }
                            Declaration::Infix(_, _) => {}
                        }
                    }
                }
                Err(e) => {
                    errors.push(e);
                }
            }
        }
        for decl in internal_declarations {
            if let Declaration::Infix(name, def) = decl {
                if let Some(mut def) = definitions.iter().find(|it| it.name == def).cloned() {
                    def.name = name.to_string();
                    definitions.push(def);
                }
            }
        }

        if errors.is_empty() {
            Ok((declarations, definitions))
        } else {
            Err(errors)
        }
    }

    fn get_exposed_decls(all_decls: &Vec<Declaration>, exposed: &Vec<Exposing>) -> Result<Vec<Declaration>, InterpreterError> {
        let mut exposed_decls = Vec::new();

        for exp in exposed.iter() {
            match exp {
                Exposing::Adt(name, adt_exp) => {
                    match adt_exp {
                        AdtExposing::Variants(variants) => {
                            for it in all_decls.iter() {
                                let (variant_name, ty) = match it {
                                    Declaration::Definition(variant_name, def) => (variant_name, &def.header),
                                    Declaration::Port(variant_name, ty) => (variant_name, ty),
                                    _ => continue
                                };
                                if variants.contains(variant_name) {
                                    if let Type::Tag(tag_name, _) = Self::get_fun_return(ty) {
                                        if &tag_name == name {
                                            exposed_decls.push(it.clone());
                                        }
                                    }
                                }
                            }
                        }
                        AdtExposing::All => {
                            for it in all_decls.iter() {
                                let ty = match it {
                                    Declaration::Definition(_, def) => &def.header,
                                    Declaration::Port(_, ty) => ty,
                                    _ => continue
                                };
                                if let Type::Tag(tag_name, _) = Self::get_fun_return(ty) {
                                    if &tag_name == name {
                                        exposed_decls.push(it.clone());
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
                        .ok_or_else(|| InterpreterError::MissingExposing(name.clone(), all_decls.clone()))?;

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
                        .ok_or_else(|| InterpreterError::MissingExposing(name.clone(), all_decls.clone()))?;

                    exposed_decls.push(decl);
                }
                Exposing::BinaryOperator(name) => {
                    let decl = all_decls.iter()
                        .find(|decl| {
                            if let Declaration::Definition(def_name, _) = decl {
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
                            if let Declaration::Definition(def_name, _) = decl {
                                def_name == name
                            } else {
                                false
                            }
                        })
                        .map(|decl| decl.clone())
                        .ok_or_else(|| InterpreterError::MissingExposing(name.clone(), all_decls.clone()))?;

                    exposed_decls.push(decl);
                }
            }
        }

        Ok(exposed_decls)
    }

    fn get_fun_return(ty: &Type) -> Type {
        if let Type::Fun(_, ty) = ty {
            Self::get_fun_return(&*ty)
        } else {
            ty.clone()
        }
    }
}