use analyzer::static_env::StaticEnv;
use std::collections::HashMap;
use types::FunCall;
use types::FunId;
use ast::Type;
use types::Value;
use util::build_fun_type;
use util::builtin_fun_of;
use util::OptionExt;
use util::StringConversion;
use interpreter::builtins::*;

#[derive(Clone, Debug, PartialEq)]
pub struct DynamicEnv {
    pub types: StaticEnv,
    values: Vec<HashMap<String, Value>>,
    next_fun_id: FunId,
    cache: HashMap<FunCall, Value>,
}

impl DynamicEnv {
    pub fn new() -> Self {
        DynamicEnv {
            types: StaticEnv::new(),
            values: vec![HashMap::new()],
            next_fun_id: 0,
            cache: HashMap::new(),
        }
    }

    pub fn next_fun_id(&mut self) -> FunId {
        let old = self.next_fun_id;
        self.next_fun_id += 1;
        old
    }

    pub fn add(&mut self, name: &str, val: Value, ty: Type) {
        self.types.add_definition(name, ty);
        self.values.last_mut().unwrap().insert(name.to_owned(), val);
    }

    pub fn find(&self, name: &str) -> Option<(Value, Type)> {
        for map in self.values.iter().rev() {
            let opt = map.get(name).cloned();
            if let Some(_) = &opt {
                return opt.zip(self.types.find_definition(name));
            }
        }
        None
    }

    pub fn enter_block(&mut self) {
        self.types.enter_block();
        self.values.push(HashMap::new());
    }

    pub fn exit_block(&mut self) {
        self.types.exit_block();
        self.values.pop().expect("Tried to pop the global environment");
    }

    pub fn get_from_cache(&self, call: &FunCall) -> Option<Value> {
        self.cache.get(call).cloned()
    }

    pub fn add_to_cache(&mut self, call: FunCall, value: Value) {
        self.cache.insert(call, value);
    }

    pub fn default_lang_env() -> DynamicEnv {
        let mut env = DynamicEnv::new();

        let num_ty = build_fun_type(&vec![
            Type::Var("number".s()), Type::Var("number".s()), Type::Var("number".s())
        ]);
        let int_ty = build_fun_type(&vec![
            Type::Tag("Int".s(), vec![]), Type::Tag("Int".s(), vec![]), Type::Tag("Int".s(), vec![])
        ]);
        let float_ty = build_fun_type(&vec![
            Type::Tag("Float".s(), vec![]), Type::Tag("Float".s(), vec![]), Type::Tag("Float".s(), vec![])
        ]);

        let fun = builtin_fun_of(env.next_fun_id(), Box::new(builtin_add), num_ty.clone());
        env.add("+", fun, num_ty.clone());
        let fun = builtin_fun_of(env.next_fun_id(), Box::new(builtin_sub), num_ty.clone());
        env.add("-", fun, num_ty.clone());
        let fun = builtin_fun_of(env.next_fun_id(), Box::new(builtin_times), num_ty.clone());
        env.add("*", fun, num_ty.clone());
        let fun = builtin_fun_of(env.next_fun_id(), Box::new(builtin_float_div), float_ty.clone());
        env.add("/", fun, float_ty.clone());
        let fun = builtin_fun_of(env.next_fun_id(), Box::new(builtin_int_div), int_ty.clone());
        env.add("//", fun, int_ty.clone());

        env
    }
}