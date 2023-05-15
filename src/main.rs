use std::env;

use crate::helpers::error::CustomError;
use crate::structs::coffee_maker::CoffeeMaker;

mod helpers;
mod structs;

/// Espera un argumento que sea el path del archivo que se utilizará para leer las ordenes.
/// Si no se puede leer el argumento se devuelve el error [`CustomError::InvalidOrderFilePath`]
fn main() -> Result<(), CustomError> {
    let args: Vec<String> = env::args().collect();
    if let Some(file_path) = args.get(1) {
        let coffee_maker = CoffeeMaker::new();
        coffee_maker.take_orders(file_path)
    } else {
        println!("No se específico el archivo de pedidos.");
        Err(CustomError::InvalidOrderFilePath)
    }
}
