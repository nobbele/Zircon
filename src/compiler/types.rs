#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum ShortRegister {
    A,
    B,
    C,
    D,
    E,
    F,
    H,
    L,
    I,
    R,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum LongRegister {
    AF,
    BC,
    DE,
    HL,
    PC,
    SP,

    IX,
    IY,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Register {
    Short(ShortRegister),
    Long(LongRegister),
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum DataTarget {
    Register(Register),

    // TODO these should take some kind of `Expr` object to support more complex expressions
    // TODO e.g `(Table + 10)*`
    Address(u16),
    Immediate(u16),

    // TODO move these into Address and Immediate?
    IdentifierImmediate(String),
    IdentifierAddress(String),
}
