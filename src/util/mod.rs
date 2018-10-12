use tokenizer::Token;
use types::*;

pub mod name_sequence;
pub mod format;
pub mod expression_fold;

#[cfg(test)]
macro_rules! assert_ok {
   ($r: expr, $tk: expr) => {
       match &$r {
           Ok((rem, item)) => {
               assert_eq!(*item, $tk, "Remaining: {:?}", rem);
           }
           Err(_) => {
               panic!("{:?}", $r);
           }
       }
   }
}

pub trait StringConversion {
    fn s(&self) -> String;
}

impl StringConversion for str {
    fn s(&self) -> String {
        self.to_string()
    }
}

pub trait VecExt<A> {
    fn map<B, F: FnMut(&A) -> B>(&self, f: F) -> Vec<B>;
    fn join_vec(&self, other: &[A]) -> Vec<A>;
}

impl <A: Clone> VecExt<A> for Vec<A> {
    fn map<B, F: FnMut(&A) -> B>(&self, f: F) -> Vec<B> {
        self.iter().map(f).collect()
    }

    fn join_vec(&self, other: &[A]) -> Vec<A> {
        let mut res: Vec<A> = Vec::new();
        for a in self {
            res.push(a.clone());
        }
        for b in other {
            res.push(b.clone());
        }
        res
    }
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


pub fn to_string(v: &[u8]) -> String {
    v.into_iter().map(|c| *c as char).collect::<String>()
}

pub fn create_vec<T>(first: T, rest: Vec<T>) -> Vec<T> {
    let mut vec: Vec<T> = Vec::new();
    vec.push(first);
    for i in rest.into_iter() {
        vec.push(i);
    }
    vec
}

pub fn create_vec_inv<T: Clone>(start: &Vec<T>, last: T) -> Vec<T> {
    let mut vec: Vec<T> = start.clone();
    vec.push(last);
    vec
}

pub fn parse_int(negative: bool, digits: Vec<char>) -> Int {
    let s: String = digits.into_iter().collect();
    let value = s.parse::<Int>().unwrap();
    if negative { -value } else { value }
}

pub fn parse_float(integer_part: Vec<char>, decimal_part: Vec<char>) -> Float {
    let int_part: String = integer_part.into_iter().collect();
    let dec_part: String = decimal_part.into_iter().collect();
    format!("{}.{}", int_part, dec_part).parse::<Float>().unwrap()
}

pub fn parse_float2(minus: bool, integer_part: Vec<char>, decimal_part: Vec<char>) -> Float {
    let int_part: String = integer_part.into_iter().collect();
    let dec_part: String = decimal_part.into_iter().collect();
    let value = format!("{}.{}", int_part, dec_part).parse::<Float>().unwrap();
    if minus { -value } else { value }
}

pub fn build_fun_type(types: &[Type]) -> Type {
    assert!(types.len() >= 2);

    if types.len() == 2 {
        Type::Fun(
            Box::from(types[0].clone()),
            Box::from(types[1].clone()),
        )
    } else {
        Type::Fun(
            Box::from(types[0].clone()),
            Box::from(build_fun_type(&types[1..])),
        )
    }
}