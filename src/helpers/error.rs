#![allow(dead_code)]
/// Errores del programa. Necesito el [`allow(dead_code)`] porque el  último error lo estoy utilizando en los tests.
#[derive(Debug, PartialEq, Eq)]
pub enum CustomError {
    /// El archivo de pedidos no se pudo abrir.
    CantOpenOrderFile,
    /// El path del archivo de pedidos es invalido.
    InvalidOrderFilePath,
    /// Algun valor en la linea de un pedido es invalido.
    InvalidOrderValue,
    /// No se puede obtener lock. Proviene de td::sync::PoisonError
    PoisonedLock,
    /// No se pudo enviar la señal de apagado a todos los procesos.
    InvalidShutDown,
    /// No hay suficiente cantidad de algún ingrediente para satisfacer un pedido.
    InsufficientIngredients,
    /// Fallo el test por un error de ejecucion.
    TestFailing,
}

impl From<std::num::ParseIntError> for CustomError {
    fn from(_: std::num::ParseIntError) -> Self {
        CustomError::InvalidOrderValue
    }
}

impl<T> From<std::sync::PoisonError<T>> for CustomError {
    fn from(_: std::sync::PoisonError<T>) -> Self {
        CustomError::PoisonedLock
    }
}
