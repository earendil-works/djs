//! Source module declarations and metadata, using the ordinary parser and resolver.
use alloc::string::String;
use alloc::vec::Vec;

use super::func::{BODY_SCOPE, FuncKind, FuncState, VarKind};
use super::parser::{Parser, syntax};
use super::{CompileError, CompileOptions, PResult};
use crate::heap::{Atom, Heap, Ref};
use crate::lexer::{Kw, TokenKind};
use crate::value::{Obj, Value};

pub(crate) struct ImportEntry {
    pub request: String,
    pub imported: String,
    pub local: Atom,
}

pub(crate) struct ModuleInfo {
    pub locals: Obj,
    pub imports: Vec<ImportEntry>,
    pub requests: Vec<String>,
    pub exports: Vec<(String, Atom)>,
    pub declaration_export: bool,
}

impl ModuleInfo {
    fn new(locals: Obj) -> Self {
        Self {
            locals,
            imports: Vec::new(),
            requests: Vec::new(),
            exports: Vec::new(),
            declaration_export: false,
        }
    }

    pub(super) fn prepare(&mut self, heap: &mut Heap, top: &mut FuncState) -> PResult<()> {
        for var in &mut top.vars {
            if var.scope != BODY_SCOPE
                || !matches!(
                    var.kind,
                    VarKind::Let | VarKind::Const | VarKind::Var | VarKind::Func
                )
            {
                continue;
            }
            let imported = self.imports.iter().any(|i| i.local == var.name);
            let initial = if var.kind == VarKind::Var {
                Value::UNDEFINED
            } else {
                Value::UNINITIALIZED
            };
            let binding = crate::vm::modules::source_binding(
                heap,
                self.locals,
                var.name,
                var.kind.is_immutable(),
                imported,
                initial,
            )?;
            var.module = Some(binding);
        }
        for (_, name) in &self.exports {
            if top.find_in_scope(BODY_SCOPE, *name).is_none() {
                return syntax(0, "export refers to an undeclared module binding");
            }
        }
        Ok(())
    }
}

pub(crate) fn compile(
    heap: &mut Heap,
    src: &[u8],
    filename: &str,
    locals: Obj,
) -> Result<(Ref, ModuleInfo), CompileError> {
    let opts = CompileOptions::eval(filename);
    let result = (|| {
        let mut parser = Parser::new(heap, src, &opts)?;
        parser.f.kind = FuncKind::Module;
        parser.completion = None;
        parser.source_module = Some(ModuleInfo::new(locals));
        let mut info = ModuleInfo::new(locals);
        let (template, ..) = parser.compile_with_module(Some(&mut info))?;
        Ok((template, info))
    })();
    result.map_err(|e| super::error_at(src, e))
}

impl Parser<'_, '_> {
    pub(super) fn module_declaration_export(&mut self, name: Atom) -> PResult<()> {
        if self.f.kind == FuncKind::Module
            && self.scope == BODY_SCOPE
            && self
                .source_module
                .as_ref()
                .is_some_and(|m| m.declaration_export)
        {
            let exported = self.heap.atom_name(name);
            self.module_export(exported, name)?;
        }
        Ok(())
    }

    fn module_export(&mut self, exported: String, local: Atom) -> PResult<()> {
        let module = self
            .source_module
            .as_mut()
            .expect("source module parser state");
        if module.exports.iter().any(|(name, _)| *name == exported) {
            return syntax(self.tok.start, "duplicate module export");
        }
        module.exports.push((exported, local));
        Ok(())
    }

    fn module_name(&mut self) -> PResult<String> {
        let name = if self.is(TokenKind::String) {
            String::from_utf16(&self.cooked_string()).map_err(|_| {
                super::PErr::syntax(
                    self.tok.start,
                    "module names with lone surrogates are not supported".into(),
                )
            })?
        } else if self.tok.kind.is_identifier_name() {
            let name = self.ident_atom()?;
            self.heap.atom_name(name)
        } else {
            return syntax(self.tok.start, "module name expected");
        };
        self.next()?;
        Ok(name)
    }

    fn module_request(&mut self) -> PResult<String> {
        if !self.is(TokenKind::String) {
            return syntax(self.tok.start, "module specifier string expected");
        }
        let request = self.module_name()?;
        let module = self
            .source_module
            .as_mut()
            .expect("source module parser state");
        if !module.requests.contains(&request) {
            module.requests.push(request.clone());
        }
        if (self.is(TokenKind::With) || self.lx.text(&self.tok) == b"assert")
            && !self.tok.newline_before()
        {
            return syntax(
                self.tok.start,
                "module import attributes are not supported yet",
            );
        }
        Ok(request)
    }

    fn module_from(&mut self) -> PResult<String> {
        if !self.tok.is_contextual(Kw::From) {
            return syntax(self.tok.start, "expecting 'from'");
        }
        self.next()?;
        self.module_request()
    }

    fn module_local(&mut self) -> PResult<Atom> {
        let name = self.binding_name()?;
        let pos = self.tok.start;
        self.declare_lexical(name, VarKind::Const, pos)?;
        self.next()?;
        Ok(name)
    }

    pub(super) fn source_module_item(&mut self) -> PResult<()> {
        if self.f.kind != FuncKind::Module || self.scope != BODY_SCOPE {
            return syntax(self.tok.start, "module declarations must be at top level");
        }
        let import = self.is(TokenKind::Import);
        self.next()?;
        if import {
            if self.is(TokenKind::String) {
                self.module_request()?;
                return self.expect_semicolon();
            }
            let mut bindings = Vec::new();
            if self.is(TokenKind::Ident) {
                bindings.push((String::from("default"), self.module_local()?));
                if !self.eat(TokenKind::Comma)? {
                    let request = self.module_from()?;
                    self.module_import_entries(request, bindings);
                    return self.expect_semicolon();
                }
            }
            if self.is(TokenKind::Star) {
                return syntax(self.tok.start, "namespace imports are not supported yet");
            }
            self.expect(TokenKind::LBrace, "{")?;
            while !self.is(TokenKind::RBrace) {
                let original = self.tok;
                let imported = self.module_name()?;
                let local = if self.tok.is_contextual(Kw::As) {
                    self.next()?;
                    self.module_local()?
                } else {
                    // Validate shorthand names as binding identifiers, including keywords.
                    let next = self.tok;
                    self.tok = original;
                    let name = self.binding_name()?;
                    self.tok = next;
                    self.declare_lexical(name, VarKind::Const, original.start)?;
                    name
                };
                bindings.push((imported, local));
                if !self.eat(TokenKind::Comma)? {
                    break;
                }
            }
            self.expect(TokenKind::RBrace, "}")?;
            let request = self.module_from()?;
            self.module_import_entries(request, bindings);
            return self.expect_semicolon();
        }
        if self.is(TokenKind::Star) {
            return syntax(self.tok.start, "star exports are not supported yet");
        }
        if self.eat(TokenKind::Default)? {
            let token = self.tok;
            // A named default function/class also declares its name in the module.
            let mut lexer = self.lx;
            let mut next = lexer.next_token()?;
            let async_function = self.at_async_function();
            if async_function {
                next = lexer.next_token()?;
            }
            if (token.kind == TokenKind::Function || async_function) && next.kind == TokenKind::Star
            {
                next = lexer.next_token()?;
            }
            if (matches!(token.kind, TokenKind::Function | TokenKind::Class) || async_function)
                && next.kind == TokenKind::Ident
            {
                self.tok = next;
                let local = self.binding_name()?;
                self.tok = token;
                self.module_export(String::from("default"), local)?;
                return self.parse_statement_list_item();
            }
            let local = self.heap.atom_str("<module default>")?;
            self.declare_lexical(local, VarKind::Const, token.start)?;
            self.module_export(String::from("default"), local)?;
            let default_name = self.heap.atom_str("default")?;
            self.parse_named(default_name, |p| p.parse_assign(false))?;
            let reference = self.init_ref(local, token.start);
            self.code().name_init(reference);
            if matches!(token.kind, TokenKind::Function | TokenKind::Class) {
                self.eat(TokenKind::Semicolon)?;
                Ok(())
            } else {
                self.expect_semicolon()
            }
        } else if self.eat(TokenKind::LBrace)? {
            let mut names = Vec::new();
            while !self.is(TokenKind::RBrace) {
                let local = self.module_name()?;
                let exported = if self.tok.is_contextual(Kw::As) {
                    self.next()?;
                    self.module_name()?
                } else {
                    local.clone()
                };
                names.push((local, exported));
                if !self.eat(TokenKind::Comma)? {
                    break;
                }
            }
            self.expect(TokenKind::RBrace, "}")?;
            let request = if self.tok.is_contextual(Kw::From) {
                Some(self.module_from()?)
            } else {
                None
            };
            for (original, exported) in names {
                let local = if let Some(request) = &request {
                    let index = self
                        .source_module
                        .as_ref()
                        .expect("module state")
                        .imports
                        .len();
                    let local = self.heap.atom_str(&alloc::format!("<reexport {index}>"))?;
                    self.declare_lexical(local, VarKind::Const, self.tok.start)?;
                    self.module_import_entries(request.clone(), alloc::vec![(original, local)]);
                    local
                } else {
                    self.heap.atom_str(&original)?
                };
                self.module_export(exported, local)?;
            }
            self.expect_semicolon()
        } else {
            if !(matches!(
                self.tok.kind,
                TokenKind::Var | TokenKind::Const | TokenKind::Function | TokenKind::Class
            ) || self.tok.is_contextual(Kw::Let)
                || self.at_async_function())
            {
                return syntax(self.tok.start, "export declaration expected");
            }
            self.source_module
                .as_mut()
                .expect("module state")
                .declaration_export = true;
            let result = self.parse_statement_list_item();
            self.source_module
                .as_mut()
                .expect("module state")
                .declaration_export = false;
            result
        }
    }

    fn module_import_entries(&mut self, request: String, bindings: Vec<(String, Atom)>) {
        let module = self.source_module.as_mut().expect("module parser state");
        for (imported, local) in bindings {
            module.imports.push(ImportEntry {
                request: request.clone(),
                imported,
                local,
            });
        }
    }
}
