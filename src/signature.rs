use crate::types::Type;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassSignature {
    pub type_parameters: Vec<TypeParameter>,
    pub super_class: ClassTypeSignature,
    pub interfaces: Vec<ClassTypeSignature>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MethodSignature {
    pub type_parameters: Vec<TypeParameter>,
    pub parameter_types: Vec<SignatureType>,
    pub return_type: SignatureType,
    pub throws: Vec<ThrowsSignature>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeParameter {
    pub name: String,
    pub class_bound: Option<SignatureType>,
    pub interface_bounds: Vec<SignatureType>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThrowsSignature {
    Class(ClassTypeSignature),
    TypeVariable(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignatureType {
    Base(Type),
    TypeVariable(String),
    Array(Box<SignatureType>),
    Class(ClassTypeSignature),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassTypeSignature {
    pub package_specifier: Vec<String>,
    pub simple_class: SimpleClassTypeSignature,
    pub suffixes: Vec<SimpleClassTypeSignature>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimpleClassTypeSignature {
    pub name: String,
    pub type_arguments: Vec<TypeArgument>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeArgument {
    Any,
    Exact(SignatureType),
    Extends(SignatureType),
    Super(SignatureType),
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum SignatureError {
    #[error("unexpected end of signature")]
    UnexpectedEof,
    #[error("invalid signature at offset {offset}: {message}")]
    Invalid { offset: usize, message: String },
}

impl SignatureType {
    pub fn base(ty: Type) -> Self {
        Self::Base(ty)
    }

    pub fn as_class(&self) -> Option<&ClassTypeSignature> {
        match self {
            SignatureType::Class(class) => Some(class),
            _ => None,
        }
    }

    fn write_to(&self, out: &mut String) {
        match self {
            SignatureType::Base(ty) => out.push(base_type_descriptor(ty)),
            SignatureType::TypeVariable(name) => {
                out.push('T');
                out.push_str(name);
                out.push(';');
            }
            SignatureType::Array(element) => {
                out.push('[');
                element.write_to(out);
            }
            SignatureType::Class(class) => class.write_to(out),
        }
    }
}

impl ClassTypeSignature {
    pub fn internal_name(&self) -> String {
        let mut name = String::new();
        if !self.package_specifier.is_empty() {
            name.push_str(&self.package_specifier.join("/"));
            name.push('/');
        }
        name.push_str(&self.simple_class.name);
        for suffix in &self.suffixes {
            name.push('$');
            name.push_str(&suffix.name);
        }
        name
    }

    fn write_to(&self, out: &mut String) {
        out.push('L');
        if !self.package_specifier.is_empty() {
            out.push_str(&self.package_specifier.join("/"));
            out.push('/');
        }
        self.simple_class.write_to(out);
        for suffix in &self.suffixes {
            out.push('.');
            suffix.write_to(out);
        }
        out.push(';');
    }
}

impl SimpleClassTypeSignature {
    fn write_to(&self, out: &mut String) {
        out.push_str(&self.name);
        if !self.type_arguments.is_empty() {
            out.push('<');
            for argument in &self.type_arguments {
                argument.write_to(out);
            }
            out.push('>');
        }
    }
}

impl TypeArgument {
    fn write_to(&self, out: &mut String) {
        match self {
            TypeArgument::Any => out.push('*'),
            TypeArgument::Exact(ty) => ty.write_to(out),
            TypeArgument::Extends(ty) => {
                out.push('+');
                ty.write_to(out);
            }
            TypeArgument::Super(ty) => {
                out.push('-');
                ty.write_to(out);
            }
        }
    }
}

impl std::fmt::Display for SignatureType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut out = String::new();
        self.write_to(&mut out);
        f.write_str(&out)
    }
}

impl std::fmt::Display for ClassTypeSignature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut out = String::new();
        self.write_to(&mut out);
        f.write_str(&out)
    }
}

impl std::fmt::Display for ClassSignature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut out = String::new();
        write_type_parameters(&self.type_parameters, &mut out);
        self.super_class.write_to(&mut out);
        for interface in &self.interfaces {
            interface.write_to(&mut out);
        }
        f.write_str(&out)
    }
}

impl std::fmt::Display for MethodSignature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut out = String::new();
        write_type_parameters(&self.type_parameters, &mut out);
        out.push('(');
        for parameter in &self.parameter_types {
            parameter.write_to(&mut out);
        }
        out.push(')');
        self.return_type.write_to(&mut out);
        for throws in &self.throws {
            out.push('^');
            match throws {
                ThrowsSignature::Class(class) => class.write_to(&mut out),
                ThrowsSignature::TypeVariable(name) => {
                    out.push('T');
                    out.push_str(name);
                    out.push(';');
                }
            }
        }
        f.write_str(&out)
    }
}

pub fn parse_class_signature(signature: &str) -> Result<ClassSignature, SignatureError> {
    let mut parser = Parser::new(signature);
    let type_parameters = parser.parse_type_parameters()?;
    let super_class = parser.parse_class_type_signature()?;
    let mut interfaces = Vec::new();
    while !parser.is_eof() {
        interfaces.push(parser.parse_class_type_signature()?);
    }
    Ok(ClassSignature {
        type_parameters,
        super_class,
        interfaces,
    })
}

pub fn parse_method_signature(signature: &str) -> Result<MethodSignature, SignatureError> {
    let mut parser = Parser::new(signature);
    let type_parameters = parser.parse_type_parameters()?;
    parser.expect(b'(')?;
    let mut parameter_types = Vec::new();
    while parser.peek()? != b')' {
        parameter_types.push(parser.parse_java_type_signature()?);
    }
    parser.expect(b')')?;
    let return_type = if parser.peek()? == b'V' {
        parser.pos += 1;
        SignatureType::Base(Type::Void)
    } else {
        parser.parse_java_type_signature()?
    };

    let mut throws = Vec::new();
    while !parser.is_eof() {
        parser.expect(b'^')?;
        let throw_type = match parser.peek()? {
            b'L' => ThrowsSignature::Class(parser.parse_class_type_signature()?),
            b'T' => {
                let name = parser.parse_type_variable_name()?;
                ThrowsSignature::TypeVariable(name)
            }
            other => {
                return Err(parser.error(format!(
                    "invalid throws signature start '{}'",
                    other as char
                )));
            }
        };
        throws.push(throw_type);
    }

    Ok(MethodSignature {
        type_parameters,
        parameter_types,
        return_type,
        throws,
    })
}

pub fn parse_field_signature(signature: &str) -> Result<SignatureType, SignatureError> {
    let mut parser = Parser::new(signature);
    let ty = parser.parse_reference_type_signature()?;
    parser.finish()?;
    Ok(ty)
}

fn write_type_parameters(type_parameters: &[TypeParameter], out: &mut String) {
    if type_parameters.is_empty() {
        return;
    }
    out.push('<');
    for parameter in type_parameters {
        out.push_str(&parameter.name);
        out.push(':');
        if let Some(bound) = &parameter.class_bound {
            bound.write_to(out);
        }
        for bound in &parameter.interface_bounds {
            out.push(':');
            bound.write_to(out);
        }
    }
    out.push('>');
}

fn base_type_descriptor(ty: &Type) -> char {
    match ty {
        Type::Void => 'V',
        Type::Boolean => 'Z',
        Type::Char => 'C',
        Type::Byte => 'B',
        Type::Short => 'S',
        Type::Int => 'I',
        Type::Float => 'F',
        Type::Long => 'J',
        Type::Double => 'D',
        _ => panic!("invalid base type for signature: {ty:?}"),
    }
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(signature: &'a str) -> Self {
        Self {
            bytes: signature.as_bytes(),
            pos: 0,
        }
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.bytes.len()
    }

    fn finish(&self) -> Result<(), SignatureError> {
        if self.is_eof() {
            Ok(())
        } else {
            Err(self.error("trailing input".to_string()))
        }
    }

    fn peek(&self) -> Result<u8, SignatureError> {
        self.bytes
            .get(self.pos)
            .copied()
            .ok_or(SignatureError::UnexpectedEof)
    }

    fn expect(&mut self, expected: u8) -> Result<(), SignatureError> {
        let actual = self.peek()?;
        if actual != expected {
            return Err(self.error(format!(
                "expected '{}', found '{}'",
                expected as char, actual as char
            )));
        }
        self.pos += 1;
        Ok(())
    }

    fn error(&self, message: String) -> SignatureError {
        SignatureError::Invalid {
            offset: self.pos,
            message,
        }
    }

    fn parse_type_parameters(&mut self) -> Result<Vec<TypeParameter>, SignatureError> {
        if self.is_eof() || self.peek()? != b'<' {
            return Ok(Vec::new());
        }
        self.pos += 1;
        let mut parameters = Vec::new();
        while self.peek()? != b'>' {
            let name = self.parse_identifier(&[b':'])?;
            self.expect(b':')?;
            let class_bound = if matches!(self.peek()?, b'L' | b'T' | b'[') {
                Some(self.parse_reference_type_signature()?)
            } else {
                None
            };
            let mut interface_bounds = Vec::new();
            while self.peek()? == b':' {
                self.pos += 1;
                interface_bounds.push(self.parse_reference_type_signature()?);
            }
            parameters.push(TypeParameter {
                name,
                class_bound,
                interface_bounds,
            });
        }
        self.expect(b'>')?;
        Ok(parameters)
    }

    fn parse_java_type_signature(&mut self) -> Result<SignatureType, SignatureError> {
        match self.peek()? {
            b'B' => {
                self.pos += 1;
                Ok(SignatureType::Base(Type::Byte))
            }
            b'C' => {
                self.pos += 1;
                Ok(SignatureType::Base(Type::Char))
            }
            b'D' => {
                self.pos += 1;
                Ok(SignatureType::Base(Type::Double))
            }
            b'F' => {
                self.pos += 1;
                Ok(SignatureType::Base(Type::Float))
            }
            b'I' => {
                self.pos += 1;
                Ok(SignatureType::Base(Type::Int))
            }
            b'J' => {
                self.pos += 1;
                Ok(SignatureType::Base(Type::Long))
            }
            b'S' => {
                self.pos += 1;
                Ok(SignatureType::Base(Type::Short))
            }
            b'Z' => {
                self.pos += 1;
                Ok(SignatureType::Base(Type::Boolean))
            }
            b'V' => {
                self.pos += 1;
                Ok(SignatureType::Base(Type::Void))
            }
            _ => self.parse_reference_type_signature(),
        }
    }

    fn parse_reference_type_signature(&mut self) -> Result<SignatureType, SignatureError> {
        match self.peek()? {
            b'L' => Ok(SignatureType::Class(self.parse_class_type_signature()?)),
            b'T' => Ok(SignatureType::TypeVariable(self.parse_type_variable_name()?)),
            b'[' => {
                self.pos += 1;
                Ok(SignatureType::Array(Box::new(
                    self.parse_java_type_signature()?,
                )))
            }
            other => Err(self.error(format!(
                "expected reference type signature, found '{}'",
                other as char
            ))),
        }
    }

    fn parse_type_variable_name(&mut self) -> Result<String, SignatureError> {
        self.expect(b'T')?;
        let name = self.parse_identifier(&[b';'])?;
        self.expect(b';')?;
        Ok(name)
    }

    fn parse_class_type_signature(&mut self) -> Result<ClassTypeSignature, SignatureError> {
        self.expect(b'L')?;
        let mut package_specifier = Vec::new();
        let mut simple_name = self.parse_identifier(&[b'/', b';', b'<', b'.'])?;
        while self.peek()? == b'/' {
            self.pos += 1;
            package_specifier.push(simple_name);
            simple_name = self.parse_identifier(&[b'/', b';', b'<', b'.'])?;
        }

        let simple_class = self.parse_simple_class_type_signature(simple_name)?;
        let mut suffixes = Vec::new();
        while self.peek()? == b'.' {
            self.pos += 1;
            let name = self.parse_identifier(&[b';', b'<', b'.'])?;
            suffixes.push(self.parse_simple_class_type_signature(name)?);
        }
        self.expect(b';')?;
        Ok(ClassTypeSignature {
            package_specifier,
            simple_class,
            suffixes,
        })
    }

    fn parse_simple_class_type_signature(
        &mut self,
        name: String,
    ) -> Result<SimpleClassTypeSignature, SignatureError> {
        let type_arguments = if !self.is_eof() && self.peek()? == b'<' {
            self.parse_type_arguments()?
        } else {
            Vec::new()
        };
        Ok(SimpleClassTypeSignature {
            name,
            type_arguments,
        })
    }

    fn parse_type_arguments(&mut self) -> Result<Vec<TypeArgument>, SignatureError> {
        self.expect(b'<')?;
        let mut arguments = Vec::new();
        while self.peek()? != b'>' {
            let argument = match self.peek()? {
                b'*' => {
                    self.pos += 1;
                    TypeArgument::Any
                }
                b'+' => {
                    self.pos += 1;
                    TypeArgument::Extends(self.parse_reference_type_signature()?)
                }
                b'-' => {
                    self.pos += 1;
                    TypeArgument::Super(self.parse_reference_type_signature()?)
                }
                _ => TypeArgument::Exact(self.parse_reference_type_signature()?),
            };
            arguments.push(argument);
        }
        self.expect(b'>')?;
        Ok(arguments)
    }

    fn parse_identifier(&mut self, delimiters: &[u8]) -> Result<String, SignatureError> {
        let start = self.pos;
        while let Some(byte) = self.bytes.get(self.pos) {
            if delimiters.contains(byte) {
                break;
            }
            self.pos += 1;
        }
        if self.pos == start {
            return Err(self.error("expected identifier".to_string()));
        }
        std::str::from_utf8(&self.bytes[start..self.pos])
            .map(|value| value.to_string())
            .map_err(|_| self.error("invalid utf-8".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ClassSignature, ClassTypeSignature, MethodSignature, SignatureType, ThrowsSignature,
        TypeArgument, parse_class_signature, parse_field_signature, parse_method_signature,
    };

    fn assert_class_roundtrip(signature: &str) -> ClassSignature {
        let parsed = parse_class_signature(signature).expect("class signature should parse");
        assert_eq!(signature, parsed.to_string());
        parsed
    }

    fn assert_field_roundtrip(signature: &str) -> SignatureType {
        let parsed = parse_field_signature(signature).expect("field signature should parse");
        assert_eq!(signature, parsed.to_string());
        parsed
    }

    fn assert_method_roundtrip(signature: &str) -> MethodSignature {
        let parsed = parse_method_signature(signature).expect("method signature should parse");
        assert_eq!(signature, parsed.to_string());
        parsed
    }

    #[test]
    fn parses_class_signature_with_inner_type_arguments() {
        let parsed =
            assert_class_roundtrip("<P:Ljava/lang/Object;>Ljava/lang/Object;Ljava/util/List<TP;>;");
        assert_eq!(parsed.type_parameters.len(), 1);
        assert_eq!(parsed.super_class.internal_name(), "java/lang/Object");
        assert_eq!(parsed.interfaces.len(), 1);
        assert_eq!(parsed.interfaces[0].internal_name(), "java/util/List");
    }

    #[test]
    fn parses_field_signature_with_parameterized_inner_class() {
        let parsed = assert_field_roundtrip(
            "Lpkg/TestParameterizedTypes<Ljava/lang/Number;>.Inner<Ljava/lang/String;>;",
        );
        let SignatureType::Class(class) = parsed else {
            panic!("expected class signature");
        };
        assert_eq!(class.internal_name(), "pkg/TestParameterizedTypes$Inner");
        assert_eq!(class.simple_class.name, "TestParameterizedTypes");
        assert_eq!(class.suffixes.len(), 1);
        assert_eq!(class.suffixes[0].name, "Inner");
    }

    #[test]
    fn parses_method_signature_with_bounds_and_throws() {
        let parsed = assert_method_roundtrip(
            "<T:Ljava/lang/Object;U::Ljava/lang/Runnable;>(TT;Ljava/util/List<+Ljava/lang/Number;>;)[TU;^Ljava/io/IOException;^TT;",
        );
        assert_eq!(parsed.type_parameters.len(), 2);
        assert!(matches!(
            parsed.throws.as_slice(),
            [
                ThrowsSignature::Class(_),
                ThrowsSignature::TypeVariable(name)
            ] if name == "T"
        ));
    }

    #[test]
    fn parses_deep_field_signature() {
        let parsed = assert_field_roundtrip("LGeneric<LOpen;LGeneric<LOpen;LGeneric<LOpen;LClose;>;LClose;>;LClose;>;");
        let SignatureType::Class(class) = parsed else {
            panic!("expected class signature");
        };
        assert_eq!(class.simple_class.name, "Generic");
    }

    #[test]
    fn rejects_invalid_signature() {
        let error = parse_method_signature("-").expect_err("signature should be rejected");
        assert!(error.to_string().contains("invalid signature") || error.to_string().contains("unexpected end"));
    }

    #[test]
    fn class_type_internal_name_uses_dollar_for_suffixes() {
        let class = ClassTypeSignature {
            package_specifier: vec!["pkg".to_string()],
            simple_class: super::SimpleClassTypeSignature {
                name: "Outer".to_string(),
                type_arguments: vec![TypeArgument::Any],
            },
            suffixes: vec![super::SimpleClassTypeSignature {
                name: "Inner".to_string(),
                type_arguments: Vec::new(),
            }],
        };
        assert_eq!(class.internal_name(), "pkg/Outer$Inner");
        assert_eq!(class.to_string(), "Lpkg/Outer<*>.Inner;");
    }
}
