use crate::arena::ExprArena;
use crate::interner::Interner;

pub struct WellKnownNames {
    pub and:   u32,
    pub or:    u32,
    pub not:   u32,
    pub count: u32,
    pub sum:   u32,
    pub avg:   u32,
    pub min:   u32,
    pub max:   u32,
}

pub struct BuildSession {
    pub interner: Interner,
    pub arena:    ExprArena,
    pub wkn:      WellKnownNames,
    fuel:         u32,
    depth:        u32,
}

const DEFAULT_FUEL:  u32 = 100_000;
const DEFAULT_DEPTH: u32 = 512;

impl BuildSession {
    pub fn new() -> Self {
        let mut interner = Interner::new();
        let wkn = WellKnownNames {
            and:   interner.intern("and"),
            or:    interner.intern("or"),
            not:   interner.intern("not"),
            count: interner.intern("count"),
            sum:   interner.intern("sum"),
            avg:   interner.intern("avg"),
            min:   interner.intern("min"),
            max:   interner.intern("max"),
        };
        Self {
            interner,
            arena: ExprArena::new(),
            wkn,
            fuel:  DEFAULT_FUEL,
            depth: DEFAULT_DEPTH,
        }
    }

    pub fn reset(&mut self) {
        self.interner.reset();
        self.arena = ExprArena::new();
        self.fuel  = DEFAULT_FUEL;
        self.depth = DEFAULT_DEPTH;
    }

    pub fn remaining_fuel(&self) -> u32 { self.fuel }
    pub fn max_depth(&self) -> u32 { self.depth }
}

impl Default for BuildSession {
    fn default() -> Self { Self::new() }
}
