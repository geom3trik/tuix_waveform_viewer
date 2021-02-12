use tuix::*;

pub struct Footer {}

impl Footer {
    pub fn new() -> Self {
        Self {}
    }
}

impl BuildHandler for Footer {
    type Ret = Entity;
    fn on_build(&mut self, state: &mut State, entity: Entity) -> Self::Ret {
        entity.set_element(state, "footer")
    }
}

impl EventHandler for Footer {}
