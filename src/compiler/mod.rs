use std::{collections::HashMap, ops::Range};

use crate::{
    tokenizer::{Token, TokenType},
    CompileError, MultiResult, Span,
};

use self::compiler_context::CompilerContext;

mod compiler_context;
mod impl_helper;
mod impl_read_tokens;
mod impl_sections;
mod types;

struct AllocatedArea {
    pub name: String,
    pub range: Range<u16>,
}

struct Compiler<'a> {
    text: &'a str,
    remaining_tokens: &'a [Token],
    latest_span: Span,
    errors: Vec<CompileError>,

    address: u16,

    allocated_areas: Vec<AllocatedArea>,

    /// Used to properly resolve late-declared identifiers.
    write_queue: Vec<Box<dyn FnOnce(&mut CompilerContext)>>,
    resolution_queue: Vec<Box<dyn Fn(&mut CompilerContext) -> bool>>,
}

impl<'a> Compiler<'a> {
    // The const generic here helps make sure the Compiler.address stays in sync with the future CompilerContext.address
    fn write<const N: usize>(&mut self, f: impl FnOnce(&mut CompilerContext) -> [u8; N] + 'static) {
        self.address += u16::try_from(N).unwrap();
        self.write_queue.push(Box::new(|ctx| {
            let data = f(ctx);
            ctx.write(&data);
        }));
    }

    // TODO check for collision
    fn set_address(&mut self, new_address: u16) {
        self.address = new_address;
        self.write_queue
            .push(Box::new(move |ctx| ctx.set_address(new_address)))
    }

    fn reserve_area(&mut self, name: &str, new_range: Range<u16>) -> Option<String> {
        for AllocatedArea { name, range } in &self.allocated_areas {
            let overlaps = (new_range.start >= range.start && new_range.start < range.end)
                || (new_range.end > range.start && new_range.end <= range.end);

            if overlaps {
                return Some(name.to_owned());
            }
        }

        self.allocated_areas.push(AllocatedArea {
            name: name.to_owned(),
            range: new_range,
        });

        None
    }

    fn resolution(&mut self, f: impl Fn(&mut CompilerContext) -> bool + 'static) {
        self.resolution_queue.push(Box::new(f));
    }

    fn compile_remaining(&mut self) {
        let token = self.peek().unwrap();
        match token.ty {
            TokenType::LabelSpecifier => self.read_label_block(),
            TokenType::DataDeclaration => self.read_data_decl(),
            TokenType::At => self.read_top_level_pragma(),
            _ => {
                self.errors.push(CompileError {
                    message: format!("Unexpected token {:?}", token.ty),
                    span: token.span,
                });
                self.next_reset();
            }
        }
    }

    pub fn compile(mut self) -> MultiResult<Vec<u8>> {
        self.skip_line_sep();

        while self.remaining_tokens.len() > 0 {
            self.compile_remaining();
            self.skip_line_sep();
        }

        if self.errors.len() > 0 {
            return MultiResult::Err(self.errors);
        }

        let mut ctx = CompilerContext {
            address: 0,
            binary: Vec::new(),
            declarations: HashMap::new(),
        };

        'resolution_loop: while self.resolution_queue.len() > 0 {
            let before = self.resolution_queue.len();
            let queue_after = self
                .resolution_queue
                .drain(..)
                .filter(|resolution| !resolution(&mut ctx))
                .collect::<Vec<_>>();
            self.resolution_queue = queue_after;
            if self.resolution_queue.len() > 0 && self.resolution_queue.len() == before {
                for _resolution in &self.resolution_queue {
                    self.errors.push(CompileError {
                        // TODO implement better message using `resolution`
                        message: format!("Could not resolve a declaration"),
                        // TODO implement correctly
                        span: Span {
                            pos: 0..1,
                            line: 0..1,
                            col: 0..1,
                        },
                    });
                }
                break 'resolution_loop;
            }
        }

        if self.errors.len() > 0 {
            return MultiResult::Err(self.errors);
        }

        for write in self.write_queue {
            write(&mut ctx);
        }

        MultiResult::Ok(ctx.binary)
    }
}

pub fn compile(text: &str, tokens: &[Token]) -> MultiResult<Vec<u8>> {
    Compiler {
        text,
        remaining_tokens: tokens,
        latest_span: Span::default(),
        errors: Vec::new(),
        write_queue: Vec::new(),
        resolution_queue: Vec::new(),
        allocated_areas: Vec::new(),
        address: 0,
    }
    .compile()
}
