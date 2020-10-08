#[derive(Debug)]
pub(super) enum Mode {
    Push,
    Pull,
    PushAndPull,
}

impl Mode {
    pub(super) fn can_push(&self) -> bool {
        match self {
            Self::Push => true,
            Self::Pull => false,
            Self::PushAndPull => true,
        }
    }

    pub(super) fn can_pull(&self) -> bool {
        match self {
            Self::Push => false,
            Self::Pull => true,
            Self::PushAndPull => true,
        }
    }
}
