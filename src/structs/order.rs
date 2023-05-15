/// Representa un pedido de un cliente.
/// Se lee del archivo indicado como parametro.
/// `<cafe molido>,<agua caliente>,<cacao>,<espuma de leche>`
#[derive(Debug)]
pub struct Order {
    /// Identificador del pedido.
    pub id: u32,
    /// Cantidad de café molido.
    pub ground_coffee: u32,
    /// Cantidad de agua caliente.
    pub hot_water: u32,
    /// Cantidad de cacao.
    pub cocoa: u32,
    /// Cantidad de espuma de leche.
    pub milk_foam: u32,
}

impl Order {
    pub fn new(
        id: u32,
        ground_coffee: u32,
        hot_water: u32,
        cocoa: u32,
        milk_foam: u32,
    ) -> Result<Order, String> {
        Ok(Order {
            id,
            ground_coffee,
            hot_water,
            cocoa,
            milk_foam,
        })
    }

    /// Transforma una línea del archivo .txt a un pedido.
    pub fn from_file_record(line: &str, id: u32) -> Result<Order, String> {
        let mut quantity_array = line.split(",");
        let ground_coffee = quantity_array
            .next()
            .ok_or("Error, no se encontró café molido en el pedido.")?
            .parse()
            .map_err(|_| "Valor inválido de café molido.")?;
        let hot_water = quantity_array
            .next()
            .ok_or("Error, no se encontró agua caliente en el pedido.")?
            .parse()
            .map_err(|_| "Valor inválido de agua caliente.")?;
        let cocoa = quantity_array
            .next()
            .ok_or("Error, no se encontró cacao en el pedido.")?
            .parse()
            .map_err(|_| "Valor inválido de cacao.")?;
        let milk_foam = quantity_array
            .next()
            .ok_or("Error, no se encontró espuma de leche en el pedido.")?
            .parse()
            .map_err(|_| "Valor inválido de espuma de leche.")?;
        Order::new(id, ground_coffee, hot_water, cocoa, milk_foam)
    }

    /// Indica si el pedido ya tiene todos sus ingredientes y puede ser entregado al cliente.
    pub fn check_if_ready(&self) -> bool {
        self.ground_coffee == 0 && self.hot_water == 0 && self.cocoa == 0 && self.milk_foam == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_order() {
        let (id, ground_coffee, hot_water, cocoa, milk_foam) = (1, 2, 3, 4, 5);
        match Order::new(id, ground_coffee, hot_water, cocoa, milk_foam) {
            Ok(order) => {
                assert_eq!(order.id, 1);
                assert_eq!(order.ground_coffee, 2);
                assert_eq!(order.hot_water, 3);
                assert_eq!(order.cocoa, 4);
                assert_eq!(order.milk_foam, 5);
            }
            Err(e) => {
                println!("[ERROR] Testeando order new: {:?}", e);
            }
        }
    }

    #[test]
    fn test_from_file_record() {
        match Order::from_file_record("2,3,4,5", 1) {
            Ok(order) => {
                assert_eq!(order.id, 1);
                assert_eq!(order.ground_coffee, 2);
                assert_eq!(order.hot_water, 3);
                assert_eq!(order.cocoa, 4);
                assert_eq!(order.milk_foam, 5);
            }
            Err(e) => {
                println!("[ERROR] Testeando from_file_record: {:?}", e);
            }
        }
    }

    #[test]
    fn test_check_if_ready() {
        match Order::from_file_record("2,3,4,5", 1) {
            Ok(order) => {
                assert_eq!(order.check_if_ready(), false);
            }
            Err(e) => {
                println!("[ERROR] Testeando from_file_record: {:?}", e);
            }
        }
        match Order::from_file_record("0,0,0,0", 1) {
            Ok(order) => {
                assert_eq!(order.check_if_ready(), true);
            }
            Err(e) => {
                println!("[ERROR] Testeando from_file_record: {:?}", e);
            }
        }
    }
}
