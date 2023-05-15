use crate::helpers::constants::C;

/// Estructura simple que actúa como contenedor de cacao.
#[derive(Debug)]
pub struct CocoaContainer {
    /// Cantidad de cacao disponible para su uso.
    pub cocoa: u32,
    /// Cantidad de cacao ya utilizado.
    pub used: u32,
}

impl CocoaContainer {
    pub fn new() -> CocoaContainer {
        CocoaContainer { cocoa: C, used: 0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_cocoa_container() {
        let cocoa_container = CocoaContainer::new();
        assert_eq!(cocoa_container.cocoa, C);
        assert_eq!(cocoa_container.used, 0);
    }
}
