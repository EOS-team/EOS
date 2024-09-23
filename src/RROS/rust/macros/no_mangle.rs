// SPDX-License-Identifier: GPL-2.0

use crate::module::{expect_group, expect_punct, try_ident};
use proc_macro::{token_stream, Delimiter, TokenStream, TokenTree};

enum BindingsTypeMapper {
    HaveBindings(String, &'static str),
    NoBindings(String),
}

#[derive(Default)]
struct NoMangleParams {
    names: Vec<String>,
    types: Vec<BindingsTypeMapper>,
    templates: Vec<Option<&'static str>>,
}

impl NoMangleParams {
    fn add_param(&mut self, name: String, ty: String) {
        let (ty, template) = match ty.trim() {
            // If the no_mangle function parameters that need to be defined
            // contain types that need to be converted from wrapper to bindings,
            // you need to add a conversion method here.
            "PtRegs" => (
                BindingsTypeMapper::HaveBindings(ty, "*mut bindings::pt_regs"),
                Some("let % = PtRegs::from_ptr(%);"),
            ),
            "IrqStage" => (
                BindingsTypeMapper::HaveBindings(ty, "*mut bindings::irq_stage"),
                Some("let % = IrqStage::from_ptr(%);"),
            ),
            "IrqWork" => (
                BindingsTypeMapper::HaveBindings(ty, "*mut bindings::irq_work"),
                Some("let % = % as *mut IrqWork;"),
            ),
            "&mut IrqWork" => (
                BindingsTypeMapper::HaveBindings(ty, "*mut bindings::irq_work"),
                Some("let % = IrqWork::from_ptr(%);"),
            ),
            "FilesStruct" => (
                BindingsTypeMapper::HaveBindings(ty, "*mut bindings::files_struct"),
                Some("let mut % = FilesStruct::from_ptr(%);"),
            ),
            "Socket" => (
                BindingsTypeMapper::HaveBindings(ty, "*mut bindings::socket"),
                Some("let mut % = Socket::from_ptr(%);"),
            ),
            _ => (BindingsTypeMapper::NoBindings(ty), None),
        };
        self.names.push(name);
        self.types.push(ty);
        self.templates.push(template);
    }

    fn parse(it: &mut token_stream::IntoIter) -> Self {
        let mut parser = NoMangleParams::default();

        while let Some(name) = try_ident(it) {
            assert_eq!(expect_punct(it), ':');

            let mut wrapper: String = String::new();
            while let Some(next) = it.next() {
                match next {
                    TokenTree::Punct(punct) => {
                        let c = punct.as_char();
                        if c == ',' {
                            break;
                        }
                        wrapper.push(c);
                    }
                    TokenTree::Ident(ident) => {
                        wrapper.push_str((ident.to_string() + " ").as_str());
                    }
                    _ => panic!("Not valid parameter."),
                }
            }
            parser.add_param(name, wrapper);
        }

        parser
    }

    fn to_parameter_group(&self) -> String {
        let mut parameters: String = String::from("(");
        for i in 0..self.names.len() {
            parameters.push_str(self.names[i].as_str());
            parameters += ": ";
            match self.types[i] {
                BindingsTypeMapper::HaveBindings(ref _w, b) => parameters.push_str(b),
                BindingsTypeMapper::NoBindings(ref t) => parameters.push_str(t),
            }
            parameters += ", ";
        }
        parameters + ")"
    }

    fn conversion_code_body(&self) -> String {
        let mut body: String = String::new();
        for i in 0..self.names.len() {
            if let Some(t) = self.templates[i] {
                body.push_str(t.replace("%", self.names[i].as_str()).as_str());
            }
        }
        body
    }
}

/// Errors about function declaration will be given by the compiler during compilation.
/// Here we only need to process the conversion of the function.
pub fn no_mangle_function_declaration(ts: TokenStream) -> TokenStream {
    let mut it = ts.into_iter();

    const FUNCTION_KEYWORDS: &[&str] = &["pub", "unsafe", "extern", "\"C\"", "fn"];

    let mut seen_keywords = Vec::new();
    let function_name: String = loop {
        match it.next() {
            None => panic!("No function name."),
            otherwise => {
                let key = otherwise.unwrap().to_string();
                if FUNCTION_KEYWORDS.contains(&key.as_str()) {
                    seen_keywords.push(key);
                } else {
                    break key;
                }
            }
        }
    };
    let keywords: String = seen_keywords.join(" ");

    let group = expect_group(&mut it);
    assert_eq!(group.delimiter(), Delimiter::Parenthesis);
    let mut group_it = group.stream().into_iter();
    let params = NoMangleParams::parse(&mut group_it);

    let (return_type, body) = match it.next() {
        Some(TokenTree::Group(group)) => (None, group.to_string()),
        Some(TokenTree::Punct(punct)) => {
            assert_eq!(punct.as_char(), '-');
            assert_eq!(expect_punct(&mut it), '>');
            let mut rtype = String::new();
            loop {
                match it.next() {
                    Some(TokenTree::Group(group)) => {
                        break (Some("->".to_string() + rtype.as_str()), group.to_string())
                    }
                    Some(TokenTree::Punct(punct)) => {
                        rtype.push(punct.as_char());
                    }
                    Some(TokenTree::Ident(ident)) => {
                        rtype.push_str((ident.to_string() + " ").as_str());
                    }
                    _ => panic!("Not valid function definition."),
                }
            }
        }
        _ => panic!("Not valid function definition."),
    };

    format!(
        "
            #[no_mangle]
            {keywords} {function_name} {parameters} {rtype} {{
                {convertion_body}
                {body}
            }}
        ",
        parameters = params.to_parameter_group(),
        convertion_body = params.conversion_code_body(),
        rtype = return_type.unwrap_or("".to_string()),
        function_name = function_name,
        body = body.strip_prefix("{").unwrap().strip_suffix("}").unwrap(),
        keywords = keywords,
    )
    .parse()
    .expect("Error parsing formatted string into token stream.")
}
