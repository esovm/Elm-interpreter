use analyzer::type_analyzer::get_type;
use analyzer::type_analyzer::TypeError;
use std::collections::HashMap;
use types::Adt;
use types::CurriedFunc;
use types::Definition;
use types::Fun;
use types::Type;
use types::Value;
use util::build_fun_type;
use util::StringConversion;

#[derive(Clone)]
pub struct Environment(Vec<Block>);

#[derive(Clone)]
struct Block {
    defs: HashMap<String, Value>,
    adts: HashMap<String, Adt>,
    alias: HashMap<String, Type>,
    variables: HashMap<String, Type>,
}

pub fn builtin_fun_of(id: u32, ty: Type) -> Value {
    Value::Fun(CurriedFunc {
        args: vec![],
        arg_count: arg_count(&ty),
        fun: Fun::Builtin(id, ty),
    })
}

pub fn arg_count(ty: &Type) -> u32 {
    match ty {
        Type::Fun(_, ref out) => {
            1 + arg_count(out)
        }
        _ => 0
    }
}

pub fn default_lang_env() -> Environment {
    let mut env = Environment::new();

//    env.add_def_type("True", &Type::Tag("Bool".s(), vec![]));
//    env.add_def_type("False", &Type::Tag("Bool".s(), vec![]));
//
//    env.add_def_type("::", &build_fun_type(&vec![
//        Type::Var("a".s()), Type::Tag("List".s(), vec![Type::Var("a".s())]), Type::Tag("List".s(), vec![Type::Var("a".s())])
//    ]));

    env.add("+", builtin_fun_of(1, build_fun_type(&vec![
        Type::Tag("number".s(), vec![]), Type::Tag("number".s(), vec![]), Type::Tag("number".s(), vec![])
    ])));
    env.add("-", builtin_fun_of(2, build_fun_type(&vec![
        Type::Tag("number".s(), vec![]), Type::Tag("number".s(), vec![]), Type::Tag("number".s(), vec![])
    ])));
    env.add("*", builtin_fun_of(3, build_fun_type(&vec![
        Type::Tag("number".s(), vec![]), Type::Tag("number".s(), vec![]), Type::Tag("number".s(), vec![])
    ])));
    env.add("/", builtin_fun_of(4, build_fun_type(&vec![
        Type::Tag("number".s(), vec![]), Type::Tag("number".s(), vec![]), Type::Tag("number".s(), vec![])
    ])));
    env.add("//", builtin_fun_of(5, build_fun_type(&vec![
        Type::Tag("number".s(), vec![]), Type::Tag("number".s(), vec![]), Type::Tag("number".s(), vec![])
    ])));

//    env.add_def_type("+", &build_fun_type(&vec![
//        Type::Tag("number".s(), vec![]), Type::Tag("number".s(), vec![]), Type::Tag("number".s(), vec![])
//    ]));
//    env.add_def_type("-", &build_fun_type(&vec![
//        Type::Tag("number".s(), vec![]), Type::Tag("number".s(), vec![]), Type::Tag("number".s(), vec![])
//    ]));
//    env.add_def_type("*", &build_fun_type(&vec![
//        Type::Tag("number".s(), vec![]), Type::Tag("number".s(), vec![]), Type::Tag("number".s(), vec![])
//    ]));
//    env.add_def_type("/", &build_fun_type(&vec![
//        Type::Tag("number".s(), vec![]), Type::Tag("number".s(), vec![]), Type::Tag("number".s(), vec![])
//    ]));

    env
}

impl Environment {
    pub fn new() -> Self {
        Environment(vec![
            Block {
                defs: HashMap::new(),
                adts: HashMap::new(),
                alias: HashMap::new(),
                variables: HashMap::new(),
            }
        ])
    }

    pub fn add(&mut self, name: &str, def: Value) {
        self.0.last_mut().unwrap().defs.insert(name.to_owned(), def);
    }

    pub fn find(&self, name: &str) -> Option<Value> {
        for b in self.0.iter().rev() {
            let opt = b.defs.get(name);

            if let Some(t) = opt {
                return Some(t.clone());
            }
        }

        None
    }

    pub fn enter_block(&mut self) {
        self.0.push(Block {
            defs: HashMap::new(),
            adts: HashMap::new(),
            alias: HashMap::new(),
            variables: HashMap::new(),
        });
    }

    pub fn exit_block(&mut self) {
        self.0.pop().expect("Tried to exit the global block");
    }
}