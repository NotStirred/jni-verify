use std::collections::{HashMap, VecDeque};

use lazy_static::lazy_static;
use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro_error::{abort, proc_macro_error};
use regex::Regex;
use syn::parse::{Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{parse_macro_input, FnArg, ItemFn, LitStr, ReturnType, Token, TypePath, TypeTuple};

struct Signature {
    span: Span,
    params: Vec<String>,
    ret: String,
}

struct Args {
    name: LitStr,
    signature: Signature,
}

lazy_static! {
    static ref SIGNATURE_REGEX: Regex = Regex::new(r"\((?<params>.*)\)(?<ret>.+)").unwrap();
    static ref TYPE_REGEX: Regex = Regex::new(r"\[?(?:[^IJBZCSFDVL;]*[IJBZCSFDV][^IJBZCSFDVL;\[]*|L[\w\.]+\;)").unwrap();
}

impl Parse for Args {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut vars = Punctuated::<LitStr, Token![,]>::parse_terminated(input)?.into_iter();

        let name = vars.next().unwrap();
        let signature = vars.next().unwrap();
        let sig = signature.value();

        let matches = SIGNATURE_REGEX
            .captures(&sig)
            .unwrap_or_else(|| abort!(signature, "Invalid signature, expected `(...)...`"));

        let params_haystack = &matches["params"];
        let ret = &matches["ret"];

        let params: Vec<_> = TYPE_REGEX
            .captures_iter(params_haystack)
            .map(|capture_match| capture_match.extract::<0>().0.to_string())
            .collect();

        Ok(Args {
            name,
            signature: Signature {span: signature.span(),
                params,
                ret: ret.to_string(),
            },
        })
    }
}

lazy_static! {
    static ref TYPE_TO_DESCRIPTOR: HashMap<&'static str, Regex> = [
        ("()", Regex::new(r"V").unwrap()),
        ("jint", Regex::new(r"I").unwrap()),
        ("jlong", Regex::new(r"J").unwrap()),
        ("jbyte", Regex::new(r"B").unwrap()),
        ("jboolean", Regex::new(r"Z").unwrap()),
        ("jchar", Regex::new(r"C").unwrap()),
        ("jshort", Regex::new(r"S").unwrap()),
        ("jfloat", Regex::new(r"F").unwrap()),
        ("jdouble", Regex::new(r"D").unwrap()),
        ("jobject", Regex::new(r"L.+;").unwrap()),
        ("jclass", Regex::new(r"Ljava.lang.Class;").unwrap()),
        ("jthrowable", Regex::new(r"Ljava.lang.Throwable;").unwrap()),
        ("jstring", Regex::new(r"Ljava.lang.String;").unwrap()),
        ("jarray", Regex::new(r"\[.+").unwrap()),
        ("jbooleanArray", Regex::new(r"\[Z").unwrap()),
        ("jbyteArray", Regex::new(r"\[B").unwrap()),
        ("jcharArray", Regex::new(r"\[C").unwrap()),
        ("jshortArray", Regex::new(r"\[S").unwrap()),
        ("jintArray", Regex::new(r"\[I").unwrap()),
        ("jlongArray", Regex::new(r"\[J").unwrap()),
        ("jfloatArray", Regex::new(r"\[F").unwrap()),
        ("jdoubleArray", Regex::new(r"\[D").unwrap()),
        ("jobjectArray", Regex::new(r"\[L.+;").unwrap()),
        ("JByteBuffer", Regex::new(r"java.nio.ByteBuffer").unwrap()),
        ("JClass", Regex::new(r"java.lang.Class").unwrap()),
        ("JList", Regex::new(r"java.lang.List").unwrap()),
        ("JMap", Regex::new(r"java.util.Map").unwrap()),
        ("JObject", Regex::new(r"L.+;").unwrap()),
        ("JObjectArray", Regex::new(r"\[java.lang.Object").unwrap()),
        ("JPrimitiveArray", Regex::new(r"\[[IJBZCSFDV]").unwrap()),
        ("JString", Regex::new(r"java.lang.String").unwrap()),
        ("JThrowable", Regex::new(r"java.lang.Throwable").unwrap()),
    ]
    .into();
}

fn type_path_as_string(path: TypePath) -> String {
    path.path.leading_colon.map_or("", |_| "::").to_string()
        + &path
            .path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<String>>()
            .join("::")
}

fn fn_arg_as_string(fnarg: &FnArg) -> String {
    match fnarg {
        syn::FnArg::Receiver(_) => unimplemented!(),
        syn::FnArg::Typed(ty) => match *ty.ty.clone() {
            syn::Type::Path(path) => type_path_as_string(path),
            _ => unimplemented!(),
        },
    }
}

fn function_sig_span(function: &ItemFn) -> Span {
    function
        .sig
        .inputs
        .first()
        .map_or(function.sig.span(), |first_input| {
            let mut span = first_input.span().clone();
            function
                .sig
                .inputs
                .iter()
                .for_each(|input| span = span.join(input.span()).unwrap_or(span));
            span
        })
}

fn ensure_function_name_is_valid_for_jni(function: &ItemFn, name: &str) {
    let function_name = function.sig.ident.to_string();

    let name_pattern = Regex::new(&(r"Java_.+".to_string() + "_" + name)).unwrap_or_else(|_|{
        abort!(
            function.sig.ident,
            "Function name {} doesn't match the java method. Expected Java_<ClassName>_{}",
            function.sig.ident.to_string(),
            name
        );
    });

    if !name_pattern.is_match(&function_name) {
        abort!(
            function.sig.ident,
            "Function name {} doesn't match the java method. Expected Java_<ClassName>_{}",
            function.sig.ident.to_string(),
            name
        );
    };
}

fn ensure_param_is(function: &ItemFn, param: Option<&FnArg>, param_type: &str) {
    let first_str = param.as_ref().map(|val| fn_arg_as_string(&val));

    if !first_str.as_ref().is_some_and(|val| val == param_type) {
        let first_span = param.map_or_else(|| function_sig_span(function), |fnarg| fnarg.span());
        abort!(
            first_span,
            "Function must have a {} parameter here, instead has {}",
            param_type,
            first_str.unwrap_or("{None}".to_string())
        );
    }
}

fn ensure_parameters_match(arguments: &[&FnArg], signature: &Signature) {
    if signature.params.len() != arguments.len() {
        abort!(
            signature.span,
            "Different number of arguments to signature ({}) and rust function ({}, excluding JNIEnv and JClass)!",
            signature.params.len(),
            arguments.len()
        );
    }

    if !arguments
        .iter()
        .map(|arg| {
            TYPE_TO_DESCRIPTOR
                .get(fn_arg_as_string(&arg).as_str())
                .unwrap_or_else(|| {
                    abort!(
                        arg,
                        "Invalid parameter type for JNI method. Can't convert type {} to descriptor",
                        fn_arg_as_string(&arg).as_str()
                    )
                })
        })
        .zip(&signature.params)
        .map(|(regex, arg)| { 
            eprintln!("{}", arg);
            regex.is_match(&arg)
        })
        .all(|b| b)
    {
        abort!(
            signature.span,
            "Parameters don't match the rust function!"
        );
    }
}

fn ensure_return_types_match(function: &ItemFn, signature: &Signature) {
    let return_type_string = match function.sig.output.clone() {
        syn::ReturnType::Default => None,
        syn::ReturnType::Type(_, ty) => Some(match *ty {
            syn::Type::Path(path) => type_path_as_string(path),
            ty => abort!(ty, "Unsupported return type"),
        }),
    }
    .unwrap_or_else(|| "()".to_string());

    if !TYPE_TO_DESCRIPTOR
        .get(return_type_string.as_str())
        .unwrap_or_else(|| {
            abort!(
                match &function.sig.output {
                    ReturnType::Type(_, ty) => ty,
                    ReturnType::Default => { 
                        // Default return type is in TYPE_TO_DESCRIPTOR map, so this must be unreachable
                        unreachable!();
                    }
                },
                "Invalid return type for JNI method. Can't convert type {} to descriptor",
                return_type_string
            )
        })
        .is_match(&signature.ret)
    {
        match &function.sig.output {
            ReturnType::Type(_, ty) => abort!(
                ty,
                "Return type `{}` doesn't match signature `{}`!",
                return_type_string.as_str(),
                signature.ret
            ),
            
            ReturnType::Default =>
                abort!(
                    function.sig.span(),
                    "Return type `{}` doesn't match signature `{}`!",
                    return_type_string.as_str(),
                    signature.ret
                )
                
        };
    }
}

#[proc_macro_error]
#[proc_macro_attribute]
pub fn verify_signature(args: TokenStream, input: TokenStream) -> TokenStream {
    let Args { name, signature } = parse_macro_input!(args as Args);
    let output = input.clone();
    let function = parse_macro_input!(input as ItemFn);

    ensure_function_name_is_valid_for_jni(&function, &name.value());

    let mut arguments = function.sig.inputs.iter().collect::<VecDeque<_>>();

    ensure_param_is(&function, arguments.pop_front(), "JNIEnv");
    ensure_param_is(&function, arguments.pop_front(), "JClass");

    ensure_parameters_match(&arguments.make_contiguous(), &signature);

    ensure_return_types_match(&function, &signature);

    output
}
