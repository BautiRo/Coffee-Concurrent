extern crate std_semaphore;

use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

use crate::helpers::constants::{
    C, SERVE_COCOA_TIME, SERVE_COFFEE_TIME, SERVE_HOT_WATER_TIME, SERVE_MILK_FOAM_TIME,
    TAKE_ORDER_TIME, TIME_TO_STATS, X,
};
use crate::helpers::error::CustomError;
use crate::helpers::file_reader;
use crate::structs::cocoa_container::CocoaContainer;
use crate::structs::coffee_container::CoffeeContainer;
use crate::structs::hot_water_container::HotWaterContainer;
use crate::structs::milk_container::MilkContainer;
use crate::structs::order::Order;
use crate::structs::statistics_values::StatisticsValues;

/// Estructura principal del programa.
pub struct CoffeeMaker {
    /// Contenedor de café molido y granos de café.
    coffee_container: Arc<(Mutex<CoffeeContainer>, Condvar)>,
    /// Contenedor de agua caliente. Tambien encargado de calentar agua de la red.
    hot_water_container: Arc<(Mutex<HotWaterContainer>, Condvar)>,
    /// Contenedor de cacao.
    cocoa_container: Arc<(Mutex<CocoaContainer>, Condvar)>,
    /// Contenedor de espuma de leche y leche fría.
    milk_container: Arc<(Mutex<MilkContainer>, Condvar)>,
    /// Contiene datos utilizados para las estadísticas.
    statistics_values: Arc<Mutex<StatisticsValues>>,
}

impl CoffeeMaker {
    pub fn new() -> CoffeeMaker {
        CoffeeMaker {
            coffee_container: Arc::new((Mutex::new(CoffeeContainer::new()), Condvar::new())),
            hot_water_container: Arc::new((Mutex::new(HotWaterContainer::new()), Condvar::new())),
            cocoa_container: Arc::new((Mutex::new(CocoaContainer::new()), Condvar::new())),
            milk_container: Arc::new((Mutex::new(MilkContainer::new()), Condvar::new())),
            statistics_values: Arc::new(Mutex::new(StatisticsValues::new())),
        }
    }

    /// Lee las líneas del archivo y las interpreta como órdenes.
    /// Si alguna linea falla la ejecución continuará sin preparar ese pedido erróneo.
    /// Para cada una de ellas abre un hilo para prepar la misma.
    /// Crea 3 hilos para los contenedores que deben ser rellenados dadas ciertas condiciones.
    /// Crea un último hilo que se encargara de la impresion de las estadísiticas.
    /// Una vez que finalizan todos los pedidos le envía una señal a los contenedores de rellenado para que dejen de correr.
    /// Errores:
    /// * Si no se puede abrir el archivo fallará con error [`CustomError::CantOpenOrderFile`]
    /// * Si no se puede enviar la señal a los contenedores de rellenado falla con [`CustomError::InvalidShutDown`] porque sino la ejecución no terminaría nunca.
    pub fn take_orders(&self, file_path: &str) -> Result<(), CustomError> {
        let lines = file_reader::read_lines(file_path);
        let mut id: u32 = 0;
        match lines {
            Ok(lines) => {
                let mut order_handle: Vec<JoinHandle<()>> = Vec::new();
                let mut refills_handle: Vec<JoinHandle<()>> = Vec::new();

                for line in lines {
                    match line {
                        Ok(line) => {
                            thread::sleep(Duration::from_millis(TAKE_ORDER_TIME));
                            match Order::from_file_record(&line, id) {
                                Ok(order) => {
                                    order_handle.push(self.prepare_order(order));
                                    id += 1;
                                }
                                Err(e) => {
                                    println!("[ERROR] No se pudo crear el pedido: {:?}", e);
                                }
                            }
                        }
                        Err(e) => {
                            println!("[ERROR] El pedido no pudo ser procesado: {:?}", e);
                        }
                    }
                }

                let coffee_container_clone = self.coffee_container.clone();
                refills_handle.push(thread::spawn(move || {
                    if CoffeeContainer::grind_coffee(coffee_container_clone).is_err() {
                        println!("[ERROR] Error en sistema al rellenar contenedor de café molido.");
                    }
                }));

                let milk_container_clone = self.milk_container.clone();
                refills_handle.push(thread::spawn(move || {
                    if MilkContainer::make_milk_foam(milk_container_clone).is_err() {
                        println!(
                            "[ERROR] Error en sistema al rellenar contenedor de espuma de leche."
                        );
                    }
                }));

                let hot_water_container_clone = self.hot_water_container.clone();
                refills_handle.push(thread::spawn(move || {
                    if HotWaterContainer::heat_water(hot_water_container_clone).is_err() {
                        println!(
                            "[ERROR] Error en sistema al rellenar contenedor de agua caliente."
                        );
                    }
                }));

                let coffee_container_clone = self.coffee_container.clone();
                let hot_water_container_clone = self.hot_water_container.clone();
                let cocoa_container_clone = self.cocoa_container.clone();
                let milk_container_clone = self.milk_container.clone();
                let statistics_values_clone = self.statistics_values.clone();
                let statistics_handle = thread::spawn(move || {
                    if Self::show_statistics(
                        coffee_container_clone,
                        hot_water_container_clone,
                        cocoa_container_clone,
                        milk_container_clone,
                        statistics_values_clone,
                    )
                    .is_err()
                    {
                        println!("[ERROR] Fallo el procesamiento de las estadísticas. Continua la preparación de pedidos sin ellas.");
                    }
                });

                for order_thread in order_handle {
                    if order_thread.join().is_err() {
                        println!("[ERROR] No se pudo unir el hilo de una orden.");
                    }
                }

                if self.send_shutdown_signal().is_err() {
                    println!("[ERROR] No se pudo enviar la señal de apagado a los contenedores.\nTerminando proceso con error.");
                    return Err(CustomError::InvalidShutDown);
                }

                for refill_thread in refills_handle {
                    if refill_thread.join().is_err() {
                        println!("[ERROR] No se pudo unir el hilo de rellenados.");
                    }
                }

                if statistics_handle.join().is_err() {
                    println!("[ERROR] No se pudo unir el hilo de estadísticas.");
                }
            }
            Err(e) => {
                println!(
                    "[ERROR] No se puedo abrir el archivo de ordenes correctamente: {:?}",
                    e
                );
                return Err(CustomError::CantOpenOrderFile);
            }
        }
        Ok(())
    }

    /// En un hilo nuevo intentará servir todos los ingredientes que correspondan con sus respectivos contenedores.
    /// Si no puede utilizar uno, ira por otro ingrediente para luego volver y asi no perder tiempo.
    /// Devuelve un [`JoinHandle`] que luego sera utilizado para finalizar el programa.
    fn prepare_order(&self, mut order: Order) -> JoinHandle<()> {
        let coffee_container_clone = self.coffee_container.clone();
        let hot_water_container_clone = self.hot_water_container.clone();
        let cocoa_container_clone = self.cocoa_container.clone();
        let milk_container_clone = self.milk_container.clone();
        let statistics_values_clone = self.statistics_values.clone();

        thread::spawn(move || {
            let mut ready = false;
            while !ready {
                if order.ground_coffee > 0 {
                    match Self::try_serve_ground_coffee(&mut order, coffee_container_clone.clone())
                    {
                        Ok(_) => {
                            ready = order.check_if_ready();
                        }
                        Err(e) => {
                            println!(
                                "[ERROR] Pedido {:?} no podrá ser preparado: {:?}",
                                order.id, e
                            );
                            break;
                        }
                    }
                }

                if order.hot_water > 0 {
                    match Self::try_serve_hot_water(&mut order, hot_water_container_clone.clone()) {
                        Ok(_) => {
                            ready = order.check_if_ready();
                        }
                        Err(e) => {
                            println!(
                                "[ERROR] No se pudo servir agua caliente para el pedido: {:?}.",
                                order.id
                            );
                            println!(
                                "[ERROR] Pedido {:?} no podrá ser preparado: {:?}",
                                order.id, e
                            );
                            break;
                        }
                    }
                }

                if order.cocoa > 0 {
                    match Self::try_serve_cocoa(&mut order, cocoa_container_clone.clone()) {
                        Ok(_) => {
                            ready = order.check_if_ready();
                        }
                        Err(e) => {
                            println!(
                                "[ERROR] Pedido {:?} no podrá ser preparado: {:?}",
                                order.id, e
                            );
                            break;
                        }
                    }
                }

                if order.milk_foam > 0 {
                    match Self::try_serve_milk_foam(&mut order, milk_container_clone.clone()) {
                        Ok(_) => {
                            ready = order.check_if_ready();
                        }
                        Err(e) => {
                            println!(
                                "[ERROR] Pedido {:?} no podrá ser preparado: {:?}",
                                order.id, e
                            );
                            break;
                        }
                    }
                }
            }
            match statistics_values_clone.lock() {
                Ok(mut statistics_values_lock) => {
                    statistics_values_lock.orders_served += 1;
                }
                Err(e) => {
                    println!("[ERROR] No se pudo entregar el pedido finalizado: {:?}", e);
                }
            }
            println!("[DEBUG] Pedido listo id:{:?}", order.id);
        })
    }

    /// Si el lock del contenedor esta disponible y el mismo tiene la capacidad para servirle café molido, le sirve.
    /// Actualiza las referencias de disponibilidades y cantidades.
    ///
    /// En caso de que el lock del contenedor de café esté envenenado en la segunda oportunidad, devuevle [`CustomError::PoisonedLock`]
    /// Si el contenedor no tiene la capacidad, entre su disponibilidad y reservas, de satisfacer el pedido, devuelve [`CustomError::InsufficientIngredients`]
    fn try_serve_ground_coffee(
        order: &mut Order,
        coffee_container: Arc<(Mutex<CoffeeContainer>, Condvar)>,
    ) -> Result<(), CustomError> {
        if order.ground_coffee == 0 {
            return Ok(());
        }
        let (coffee_lock, coffee_cvar) = &*coffee_container;
        match coffee_lock.try_lock() {
            Ok(temp_lock) => {
                if temp_lock.coffee_grains_container + temp_lock.ground_coffee_container
                    < order.ground_coffee
                {
                    println!(
                        "[ERROR] No hay suficiente café para realizar este pedido. Pedido: {:?}",
                        order.id
                    );
                    drop(temp_lock);
                    return Err(CustomError::InsufficientIngredients);
                }
                drop(temp_lock);
                if let Ok(mut state) =
                    coffee_cvar.wait_while(coffee_lock.lock()?, |coffee_container| {
                        coffee_container.ground_coffee_container < order.ground_coffee
                    })
                {
                    thread::sleep(Duration::from_millis(SERVE_COFFEE_TIME));
                    state.ground_coffee_container -= order.ground_coffee;
                    state.ground_coffee_used += order.ground_coffee;
                    println!("[DEBUG] Café servido Pedido:{:?}", order.id);
                    coffee_cvar.notify_all();
                    order.ground_coffee = 0;
                }
            }
            Err(_) => {}
        };
        Ok(())
    }

    /// Si el lock del contenedor esta disponible y el mismo tiene la capacidad para servirle agua caliente, le sirve.
    /// Actualiza las referencias de disponibilidades y cantidades.
    ///
    /// En caso de que el lock del contenedor de agua caliente esté envenenado en la segunda oportunidad, devuevle [`CustomError::PoisonedLock`]
    fn try_serve_hot_water(
        order: &mut Order,
        hot_water_container: Arc<(Mutex<HotWaterContainer>, Condvar)>,
    ) -> Result<(), CustomError> {
        if order.hot_water == 0 {
            return Ok(());
        }
        let (h_w_lock, h_w_cvar) = &*hot_water_container;
        match h_w_lock.try_lock() {
            Ok(temp_lock) => {
                drop(temp_lock);
                if let Ok(mut state) = h_w_cvar.wait_while(h_w_lock.lock()?, |h_w_container| {
                    h_w_container.hot_water < order.hot_water
                }) {
                    thread::sleep(Duration::from_millis(SERVE_HOT_WATER_TIME));
                    state.hot_water -= order.hot_water;
                    state.used += order.hot_water;
                    println!("[DEBUG] Agua caliente servida Pedido:{:?}", order.id);
                    h_w_cvar.notify_all();
                    order.hot_water = 0;
                }
            }
            Err(_) => {}
        };
        Ok(())
    }

    /// Si el lock del contenedor esta disponible y el mismo tiene la capacidad para servirle cacao, le sirve.
    /// Actualiza las referencias de disponibilidades y cantidades.
    /// Al llegar al [`X%`] de la disponibilidad de cacao se alerta por pantalla.
    ///
    /// En caso de que el lock del contenedor de cacao este envenenado en la segunda oportunidad, devuevle [`CustomError::PoisonedLock`]
    /// Si el contenedor no tiene la capacidad de satisfacer el pedido, devuelve [`CustomError::InsufficientIngredients`]
    fn try_serve_cocoa(
        order: &mut Order,
        cocoa_container: Arc<(Mutex<CocoaContainer>, Condvar)>,
    ) -> Result<(), CustomError> {
        if order.cocoa == 0 {
            return Ok(());
        }
        let (cocoa_lock, cococa_cvar) = &*cocoa_container;
        match cocoa_lock.try_lock() {
            Ok(temp_lock) => {
                if temp_lock.cocoa < order.cocoa {
                    println!(
                        "[ERROR] No hay suficiente cacao para realizar este pedido. Pedido: {:?}",
                        order.id
                    );
                    drop(temp_lock);
                    return Err(CustomError::InsufficientIngredients);
                }
                drop(temp_lock);
                if let Ok(mut state) = cococa_cvar
                    .wait_while(cocoa_lock.lock()?, |cocoa_container| {
                        cocoa_container.cocoa < order.cocoa
                    })
                {
                    thread::sleep(Duration::from_millis(SERVE_COCOA_TIME));
                    state.cocoa -= order.cocoa;
                    state.used += order.cocoa;
                    let capacity_percentage = X as f32 / 100.0 * C as f32;
                    if (state.cocoa as f32) < capacity_percentage {
                        println!("[WARN] El contenedor de cacao se encuentra por debajo de {:?}% de su capacidad", X);
                    }
                    println!("[DEBUG] Cacao servido Pedido:{:?}", order.id);
                    cococa_cvar.notify_all();
                    order.cocoa = 0;
                }
            }
            Err(_) => {}
        };
        Ok(())
    }

    /// Si el lock del contenedor esta disponible y el mismo tiene la capacidad para servirle espuma de leche, le sirve.
    /// Actualiza las referencias de disponibilidades y cantidades.
    ///
    /// En caso de que el lock del contenedor de leche esté envenenado en la segunda oportunidad, devuevle [`CustomError::PoisonedLock`]
    /// Si el contenedor no tiene la capacidad, entre su disponibilidad y reservas, de satisfacer el pedido, devuelve [`CustomError::InsufficientIngredients`]
    fn try_serve_milk_foam(
        order: &mut Order,
        milk_container: Arc<(Mutex<MilkContainer>, Condvar)>,
    ) -> Result<(), CustomError> {
        if order.milk_foam == 0 {
            return Ok(());
        }
        let (milk_lock, milk_cvar) = &*milk_container;
        match milk_lock.try_lock() {
            Ok(temp_lock) => {
                if temp_lock.milk_foam_container + temp_lock.cold_milk_container < order.milk_foam {
                    println!(
                        "[ERROR] No hay suficiente leche para realizar este pedido. Pedido: {:?}",
                        order.id
                    );
                    drop(temp_lock);
                    return Err(CustomError::InsufficientIngredients);
                }
                drop(temp_lock);
                if let Ok(mut state) = milk_cvar.wait_while(milk_lock.lock()?, |milk_container| {
                    milk_container.milk_foam_container < order.milk_foam
                }) {
                    thread::sleep(Duration::from_millis(SERVE_MILK_FOAM_TIME));
                    state.milk_foam_container -= order.milk_foam;
                    state.milk_foam_used += order.milk_foam;
                    println!("[DEBUG] Espuma de leche servida Pedido:{:?}", order.id);
                    milk_cvar.notify_all();
                    order.milk_foam = 0;
                }
            }
            Err(_) => {}
        };
        Ok(())
    }

    /// Cambia el flag [`shutdown`] de los contenedores para que los mismos puedan terminar su ejecución.
    ///
    /// Si no se consigue algún lock, se devuelve [`CustomError:PoisonedLock`] ya que sino nunca terminaría la ejecución.
    fn send_shutdown_signal(&self) -> Result<(), CustomError> {
        let (coffee_lock, coffee_cvar) = &*self.coffee_container;
        match coffee_lock.lock() {
            Ok(mut coffee_lock) => {
                coffee_lock.shutdown = true;
            }
            Err(e) => {
                println!(
                    "[ERROR] No se pudo obtener lock para apagar contenedor de café: {:?}",
                    e
                );
                return Err(CustomError::PoisonedLock);
            }
        }
        coffee_cvar.notify_all();

        let (milk_lock, milk_cvar) = &*self.milk_container;
        match milk_lock.lock() {
            Ok(mut milk_lock) => {
                milk_lock.shutdown = true;
            }
            Err(e) => {
                println!(
                    "[ERROR] No se pudo obtener lock para apagar contenedor de leche: {:?}",
                    e
                );
                return Err(CustomError::PoisonedLock);
            }
        }
        milk_cvar.notify_all();

        let (water_lock, water_cvar) = &*self.hot_water_container;
        match water_lock.lock() {
            Ok(mut water_lock) => {
                water_lock.shutdown = true;
            }
            Err(e) => {
                println!(
                    "[ERROR] No se pudo obtener lock para apagar contenedor de agua: {:?}",
                    e
                );
                return Err(CustomError::PoisonedLock);
            }
        }
        water_cvar.notify_all();

        let statistics_values_clone = self.statistics_values.clone();
        match statistics_values_clone.lock() {
            Ok(mut s_v_lock) => {
                s_v_lock.shutdown = true;
            }
            Err(e) => {
                println!(
                    "[ERROR] No se pudo obtener lock para apagar reproductor de estadísticas: {:?}",
                    e
                );
                return Err(CustomError::PoisonedLock);
            }
        }

        Ok(())
    }

    /// Se recolectan y mustran las estadísticas cada [`TIME_TO_STATS`] milisegundos.
    /// Si algún lock falla, se continúa el ciclo por lo que no se imprimirán estadísticas esta vez, sí la siguiente.
    fn show_statistics(
        coffee_container: Arc<(Mutex<CoffeeContainer>, Condvar)>,
        hot_water_container: Arc<(Mutex<HotWaterContainer>, Condvar)>,
        cocoa_container: Arc<(Mutex<CocoaContainer>, Condvar)>,
        milk_container: Arc<(Mutex<MilkContainer>, Condvar)>,
        statistics_values: Arc<Mutex<StatisticsValues>>,
    ) -> Result<(), CustomError> {
        loop {
            thread::sleep(Duration::from_millis(TIME_TO_STATS));
            let (
                grains_used,
                cold_milk_used,
                cocoa_used,
                coffee_used,
                foam_used,
                water_used,
                grains,
                cold_milk,
                cocoa,
                coffee,
                foam,
                water,
                orders_served,
            );

            let (coffee_lock, coffee_cvar) = &*coffee_container;
            match coffee_lock.lock() {
                Ok(coffee_lock) => {
                    grains_used = coffee_lock.coffee_grains_used;
                    coffee_used = coffee_lock.ground_coffee_used;
                    grains = coffee_lock.coffee_grains_container;
                    coffee = coffee_lock.ground_coffee_container;
                }
                Err(e) => {
                    println!("[ERROR] No se pudieron obtener las estadísticas: {:?}", e);
                    continue;
                }
            }
            coffee_cvar.notify_all();

            let (h_w_lock, h_w_cvar) = &*hot_water_container;
            match h_w_lock.lock() {
                Ok(h_w_lock) => {
                    water_used = h_w_lock.used;
                    water = h_w_lock.hot_water;
                }
                Err(e) => {
                    println!("[ERROR] No se pudieron obtener las estadísticas: {:?}", e);
                    continue;
                }
            }
            h_w_cvar.notify_all();

            let (cocoa_lock, cocoa_cvar) = &*cocoa_container;
            match cocoa_lock.lock() {
                Ok(cocoa_lock) => {
                    cocoa_used = cocoa_lock.used;
                    cocoa = cocoa_lock.cocoa;
                }
                Err(e) => {
                    println!("[ERROR] No se pudieron obtener las estadísticas: {:?}", e);
                    continue;
                }
            }
            cocoa_cvar.notify_all();

            let (milk_lock, milk_cvar) = &*milk_container;
            match milk_lock.lock() {
                Ok(milk_lock) => {
                    cold_milk_used = milk_lock.cold_milk_used;
                    foam_used = milk_lock.milk_foam_used;
                    cold_milk = milk_lock.cold_milk_container;
                    foam = milk_lock.milk_foam_container;
                }
                Err(e) => {
                    println!("[ERROR] No se pudieron obtener las estadísticas: {:?}", e);
                    continue;
                }
            }
            milk_cvar.notify_all();

            let shutdown;
            match statistics_values.lock() {
                Ok(statistics_values_lock) => {
                    shutdown = statistics_values_lock.shutdown;
                    orders_served = statistics_values_lock.orders_served;
                }
                Err(e) => {
                    println!("[ERROR] No se pudieron obtener las estadísticas: {:?}", e);
                    continue;
                }
            }

            println!(
                r#"
                Estadísticas:
                    Ordenes completas: {:?}
                    Café:
                        Granos consumidos: {:?}
                        Café molido consumido: {:?}
                        Disponibilidad de granos: {:?}
                        Disponibilidad de café molido: {:?}
                    Agua caliente:
                        Consumida: {:?}
                        Disponibilidad: {:?}
                    Cacao:
                        Consumido: {:?}
                        Disponibilidad: {:?}
                    Leche:
                        Leche fría consumida: {:?}
                        Espuma de leche consumida: {:?}
                        Leche fría disponible: {:?}
                        Espuma de leche disponible: {:?}
            "#,
                orders_served,
                grains_used,
                coffee_used,
                grains,
                coffee,
                water_used,
                water,
                cocoa_used,
                cocoa,
                cold_milk_used,
                foam_used,
                cold_milk,
                foam
            );
            if shutdown {
                break;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helpers::constants::{A, E, G, L, M};

    #[test]
    fn test_try_serve_ground_coffee_serial() {
        let coffee_maker = CoffeeMaker::new();

        match Order::new(1, 20, 3, 4, 5) {
            Ok(mut order) => {
                let coffee_container_clone = coffee_maker.coffee_container.clone();
                match CoffeeMaker::try_serve_ground_coffee(&mut order, coffee_container_clone) {
                    Ok(_) => {
                        assert_eq!(order.ground_coffee, 0);
                        let (coffee_lock, _) = &*coffee_maker.coffee_container;
                        match coffee_lock.lock() {
                            Ok(coffee_lock) => {
                                assert_eq!(coffee_lock.ground_coffee_used, 20);
                                assert_eq!(coffee_lock.ground_coffee_container, M - 20);
                            }
                            Err(e) => {
                                println!("[ERROR] Testeando try_serve_ground_coffee: {:?}", e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("[ERROR] Testeando try_serve_ground_coffee: {:?}", e);
                    }
                }
            }
            Err(e) => {
                println!("[ERROR] Testeando try_serve_ground_coffee: {:?}", e);
            }
        }
    }

    #[test]
    fn test_try_serve_hot_water_serial() {
        let coffee_maker = CoffeeMaker::new();

        match Order::new(1, 2, 20, 4, 5) {
            Ok(mut order) => {
                let hot_water_container_clone = coffee_maker.hot_water_container.clone();
                match CoffeeMaker::try_serve_hot_water(&mut order, hot_water_container_clone) {
                    Ok(_) => {
                        assert_eq!(order.hot_water, 0);
                        let (hot_water_lock, _) = &*coffee_maker.hot_water_container;
                        match hot_water_lock.lock() {
                            Ok(hot_water_lock) => {
                                assert_eq!(hot_water_lock.used, 20);
                                assert_eq!(hot_water_lock.hot_water, A - 20);
                            }
                            Err(e) => {
                                println!("[ERROR] Testeando try_serve_hot_water: {:?}", e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("[ERROR] Testeando try_serve_hot_water: {:?}", e);
                    }
                }
            }
            Err(e) => {
                println!("[ERROR] Testeando try_serve_hot_water: {:?}", e);
            }
        }
    }

    #[test]
    fn test_try_serve_cocoa_serial() {
        let coffee_maker = CoffeeMaker::new();

        match Order::new(1, 2, 3, 20, 5) {
            Ok(mut order) => {
                let cocoa_container_clone = coffee_maker.cocoa_container.clone();
                match CoffeeMaker::try_serve_cocoa(&mut order, cocoa_container_clone) {
                    Ok(_) => {
                        assert_eq!(order.cocoa, 0);
                        let (cocoa_lock, _) = &*coffee_maker.cocoa_container;
                        match cocoa_lock.lock() {
                            Ok(cocoa_lock) => {
                                assert_eq!(cocoa_lock.used, 20);
                                assert_eq!(cocoa_lock.cocoa, C - 20);
                            }
                            Err(e) => {
                                println!("[ERROR] Testeando try_serve_cocoa: {:?}", e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("[ERROR] Testeando try_serve_cocoa: {:?}", e);
                    }
                }
            }
            Err(e) => {
                println!("[ERROR] Testeando try_serve_cocoa: {:?}", e);
            }
        }
    }

    #[test]
    fn test_try_serve_milk_foam_serial() {
        let coffee_maker = CoffeeMaker::new();

        match Order::new(1, 2, 3, 4, 20) {
            Ok(mut order) => {
                let milk_container_clone = coffee_maker.milk_container.clone();
                match CoffeeMaker::try_serve_milk_foam(&mut order, milk_container_clone) {
                    Ok(_) => {
                        assert_eq!(order.milk_foam, 0);
                        let (milk_lock, _) = &*coffee_maker.milk_container;
                        match milk_lock.lock() {
                            Ok(milk_lock) => {
                                assert_eq!(milk_lock.milk_foam_used, 20);
                                assert_eq!(milk_lock.milk_foam_container, E - 20);
                            }
                            Err(e) => {
                                println!("[ERROR] Testeando try_serve_milk_foam: {:?}", e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("[ERROR] Testeando try_serve_milk_foam: {:?}", e);
                    }
                }
            }
            Err(e) => {
                println!("[ERROR] Testeando try_serve_milk_foam: {:?}", e);
            }
        }
    }

    #[test]
    fn test_try_serve_ground_coffee_concurrent() {
        let coffee_maker = CoffeeMaker::new();

        let mut order_1 = Order::new(1, 10, 10, 10, 10).unwrap(); // Ya esta testeado que esto no falla.
        let mut order_2 = Order::new(2, 10, 10, 10, 10).unwrap(); // Ya esta testeado que esto no falla.

        let coffee_container_clone = coffee_maker.coffee_container.clone();
        let (coffee_lock, coffee_cvar) = &*coffee_maker.coffee_container;
        let mut thread_handles: Vec<JoinHandle<()>> = Vec::new();
        match coffee_lock.lock() {
            // Mantengo el lock para que no comiencen a hacer el llenado ahora.
            Ok(coffee_lock) => {
                assert_eq!(coffee_lock.ground_coffee_container, M);
                assert_eq!(coffee_lock.coffee_grains_container, G);
                thread_handles.push(thread::spawn(move || {
                    if CoffeeMaker::try_serve_ground_coffee(
                        &mut order_1,
                        coffee_container_clone.clone(),
                    )
                    .is_err()
                    {
                        println!(
                            "[ERROR] Testeando try_serve_ground_coffee de manera concurrente."
                        );
                    }
                    if CoffeeMaker::try_serve_ground_coffee(
                        &mut order_2,
                        coffee_container_clone.clone(),
                    )
                    .is_err()
                    {
                        println!(
                            "[ERROR] Testeando try_serve_ground_coffee de manera concurrente."
                        );
                    }
                }));
            }
            Err(e) => {
                println!(
                    "[ERROR] Testeando try_serve_ground_coffee de manera concurrente: {:?}",
                    e
                );
            }
        }
        coffee_cvar.notify_all(); // Ahora si lo libero para que ambos puedan obtener el acceso.
        for thread in thread_handles {
            if thread.join().is_err() {
                println!("[ERROR] Testeando try_serve_ground_coffee de manera concurrente.");
            }
        }
        let (coffee_lock, _) = &*coffee_maker.coffee_container;
        match coffee_lock.lock() {
            Ok(coffee_lock) => {
                assert_eq!(coffee_lock.ground_coffee_container, M - 10 * 2);
                assert_eq!(coffee_lock.coffee_grains_container, G);
            }
            Err(e) => {
                println!(
                    "[ERROR] Testeando try_serve_ground_coffee de manera concurrente: {:?}",
                    e
                );
            }
        };
    }

    #[test]
    fn test_try_serve_hot_water_concurrent() {
        let coffee_maker = CoffeeMaker::new();

        let mut order_1 = Order::new(1, 10, 10, 10, 10).unwrap(); // Ya esta testeado que esto no falla.
        let mut order_2 = Order::new(2, 10, 10, 10, 10).unwrap(); // Ya esta testeado que esto no falla.

        let hot_water_container_clone = coffee_maker.hot_water_container.clone();
        let (water_lock, water_cvar) = &*coffee_maker.hot_water_container;
        let mut thread_handles: Vec<JoinHandle<()>> = Vec::new();
        match water_lock.lock() {
            // Mantengo el lock para que no comiencen a hacer el llenado ahora.
            Ok(water_lock) => {
                assert_eq!(water_lock.hot_water, A);
                thread_handles.push(thread::spawn(move || {
                    if CoffeeMaker::try_serve_hot_water(
                        &mut order_1,
                        hot_water_container_clone.clone(),
                    )
                    .is_err()
                    {
                        println!("[ERROR] Testeando try_serve_hot_water de manera concurrente.");
                    }
                    if CoffeeMaker::try_serve_hot_water(
                        &mut order_2,
                        hot_water_container_clone.clone(),
                    )
                    .is_err()
                    {
                        println!("[ERROR] Testeando try_serve_hot_water de manera concurrente.");
                    }
                }));
            }
            Err(e) => {
                println!(
                    "[ERROR] Testeando try_serve_hot_water de manera concurrente: {:?}",
                    e
                );
            }
        }
        water_cvar.notify_all(); // Ahora si lo libero para que ambos puedan obtener el acceso.
        for thread in thread_handles {
            if thread.join().is_err() {
                println!("[ERROR] Testeando try_serve_hot_water de manera concurrente.");
            }
        }
        let (water_lock, _) = &*coffee_maker.hot_water_container;
        match water_lock.lock() {
            Ok(water_lock) => {
                assert_eq!(water_lock.hot_water, A - 10 * 2);
            }
            Err(e) => {
                println!(
                    "[ERROR] Testeando try_serve_hot_water de manera concurrente: {:?}",
                    e
                );
            }
        };
    }

    #[test]
    fn test_try_serve_cocoa_concurrent() {
        let coffee_maker = CoffeeMaker::new();

        let mut order_1 = Order::new(1, 10, 10, 10, 10).unwrap(); // Ya esta testeado que esto no falla.
        let mut order_2 = Order::new(2, 10, 10, 10, 10).unwrap(); // Ya esta testeado que esto no falla.

        let cocoa_container_clone = coffee_maker.cocoa_container.clone();
        let (cocoa_lock, cocoa_cvar) = &*coffee_maker.cocoa_container;
        let mut thread_handles: Vec<JoinHandle<()>> = Vec::new();
        match cocoa_lock.lock() {
            // Mantengo el lock para que no comiencen a hacer el llenado ahora.
            Ok(cocoa_lock) => {
                assert_eq!(cocoa_lock.cocoa, C);
                thread_handles.push(thread::spawn(move || {
                    if CoffeeMaker::try_serve_cocoa(&mut order_1, cocoa_container_clone.clone())
                        .is_err()
                    {
                        println!("[ERROR] Testeando try_serve_cocoa de manera concurrente.");
                    }
                    if CoffeeMaker::try_serve_cocoa(&mut order_2, cocoa_container_clone.clone())
                        .is_err()
                    {
                        println!("[ERROR] Testeando try_serve_cocoa de manera concurrente.");
                    }
                }));
            }
            Err(e) => {
                println!(
                    "[ERROR] Testeando try_serve_cocoa de manera concurrente: {:?}",
                    e
                );
            }
        }
        cocoa_cvar.notify_all(); // Ahora si lo libero para que ambos puedan obtener el acceso.
        for thread in thread_handles {
            if thread.join().is_err() {
                println!("[ERROR] Testeando try_serve_cocoa de manera concurrente.");
            }
        }
        let (cocoa_lock, _) = &*coffee_maker.cocoa_container;
        match cocoa_lock.lock() {
            Ok(cocoa_lock) => {
                assert_eq!(cocoa_lock.cocoa, C - 10 * 2);
            }
            Err(e) => {
                println!(
                    "[ERROR] Testeando try_serve_hot_water de manera concurrente: {:?}",
                    e
                );
            }
        };
    }

    #[test]
    fn test_try_serve_milk_foam_concurrent() {
        let coffee_maker = CoffeeMaker::new();

        let mut order_1 = Order::new(1, 10, 10, 10, 10).unwrap(); // Ya esta testeado que esto no falla.
        let mut order_2 = Order::new(2, 10, 10, 10, 10).unwrap(); // Ya esta testeado que esto no falla.

        let milk_container_clone = coffee_maker.milk_container.clone();
        let (milk_lock, coffee_cvar) = &*coffee_maker.milk_container;
        let mut thread_handles: Vec<JoinHandle<()>> = Vec::new();
        match milk_lock.lock() {
            // Mantengo el lock para que no comiencen a hacer el llenado ahora.
            Ok(milk_lock) => {
                assert_eq!(milk_lock.milk_foam_container, E);
                assert_eq!(milk_lock.cold_milk_container, L);
                thread_handles.push(thread::spawn(move || {
                    if CoffeeMaker::try_serve_milk_foam(&mut order_1, milk_container_clone.clone())
                        .is_err()
                    {
                        println!("[ERROR] Testeando try_serve_milk_foam de manera concurrente.");
                    }
                    if CoffeeMaker::try_serve_milk_foam(&mut order_2, milk_container_clone.clone())
                        .is_err()
                    {
                        println!("[ERROR] Testeando try_serve_milk_foam de manera concurrente.");
                    }
                }));
            }
            Err(e) => {
                println!(
                    "[ERROR] Testeando try_serve_milk_foam de manera concurrente: {:?}",
                    e
                );
            }
        }
        coffee_cvar.notify_all(); // Ahora si lo libero para que ambos puedan obtener el acceso.
        for thread in thread_handles {
            if thread.join().is_err() {
                println!("[ERROR] Testeando try_serve_milk_foam de manera concurrente.");
            }
        }
        let (milk_lock, _) = &*coffee_maker.milk_container;
        match milk_lock.lock() {
            Ok(milk_lock) => {
                assert_eq!(milk_lock.milk_foam_container, E - 10 * 2);
                assert_eq!(milk_lock.cold_milk_container, L);
            }
            Err(e) => {
                println!(
                    "[ERROR] Testeando try_serve_milk_foam de manera concurrente: {:?}",
                    e
                );
            }
        };
    }

    #[test]
    fn test_send_shutdown_signal() {
        let coffee_maker = CoffeeMaker::new();
        if coffee_maker.send_shutdown_signal().is_err() {
            println!("[ERROR] Testeando send_shutdown_signal.");
        }

        let (coffee_lock, _) = &*coffee_maker.coffee_container;
        match coffee_lock.lock() {
            Ok(coffee_lock) => {
                assert_eq!(coffee_lock.shutdown, true);
            }
            Err(e) => {
                println!("[ERROR] Testeando send_shutdown_signal: {:?}", e);
            }
        };

        let (water_lock, _) = &*coffee_maker.hot_water_container;
        match water_lock.lock() {
            Ok(water_lock) => {
                assert_eq!(water_lock.shutdown, true);
            }
            Err(e) => {
                println!("[ERROR] Testeando send_shutdown_signal: {:?}", e);
            }
        };

        let (milk_lock, _) = &*coffee_maker.milk_container;
        match milk_lock.lock() {
            Ok(milk_lock) => {
                assert_eq!(milk_lock.shutdown, true);
            }
            Err(e) => {
                println!("[ERROR] Testeando send_shutdown_signal: {:?}", e);
            }
        };

        match coffee_maker.statistics_values.lock() {
            Ok(s_lock) => {
                assert_eq!(s_lock.shutdown, true);
            }
            Err(e) => {
                println!("[ERROR] Testeando send_shutdown_signal: {:?}", e);
            }
        };
    }

    fn assert_all_stats(
        path: &str,
        grains_used: u32,
        cold_milk_used: u32,
        cocoa_used: u32,
        coffee_used: u32,
        foam_used: u32,
        water_used: u32,
        grains: u32,
        cold_milk: u32,
        cocoa: u32,
        coffee: u32,
        foam: u32,
        water: u32,
        orders_served: u32,
    ) {
        let coffee_maker = CoffeeMaker::new();
        let result = coffee_maker.take_orders(path);
        assert!(result.is_ok());

        let coffee_container = coffee_maker.coffee_container.0.lock().unwrap();
        let hot_water_container = coffee_maker.hot_water_container.0.lock().unwrap();
        let cocoa_container = coffee_maker.cocoa_container.0.lock().unwrap();
        let milk_container = coffee_maker.milk_container.0.lock().unwrap();
        let statistics_values = coffee_maker.statistics_values.lock().unwrap();

        assert_eq!(grains_used, coffee_container.coffee_grains_used);
        assert_eq!(coffee_used, coffee_container.ground_coffee_used);
        assert_eq!(grains, coffee_container.coffee_grains_container);
        assert_eq!(coffee, coffee_container.ground_coffee_container);
        assert_eq!(true, coffee_container.shutdown);

        assert_eq!(cocoa_used, cocoa_container.used);
        assert_eq!(cocoa, cocoa_container.cocoa);

        assert_eq!(water_used, hot_water_container.used);
        assert_eq!(water, hot_water_container.hot_water);
        assert_eq!(true, hot_water_container.shutdown);

        assert_eq!(cold_milk_used, milk_container.cold_milk_used);
        assert_eq!(foam_used, milk_container.milk_foam_used);
        assert_eq!(cold_milk, milk_container.cold_milk_container);
        assert_eq!(foam, milk_container.milk_foam_container);
        assert_eq!(true, milk_container.shutdown);

        assert_eq!(orders_served, statistics_values.orders_served);
    }

    #[test]
    fn test_take_orders_one_order() {
        assert_all_stats(
            "src/tests/one_order.txt", // path
            0,                         // grains_used
            0,                         // cold_milk_used
            10,                        //cocoa_used
            10,                        //coffee_used
            10,                        //foam_used
            10,                        //water_used
            G,                         //grains
            L,                         //cold_milk
            C - 10,                    //cocoa
            M - 10,                    //coffee
            E - 10,                    //foam
            A - 10,                    //water
            1,                         //orders_served
        );
    }

    #[test]
    fn test_take_orders_invalid_file() {
        let coffee_maker = CoffeeMaker::new();
        let result = coffee_maker.take_orders("src/tests/invalid.txt");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), CustomError::CantOpenOrderFile);
    }

    #[test]
    fn test_take_orders_empty() {
        assert_all_stats(
            "src/tests/empty.txt", // path
            0,                     // grains_used
            0,                     // cold_milk_used
            0,                     //cocoa_used
            0,                     //coffee_used
            0,                     //foam_used
            0,                     //water_used
            G,                     //grains
            L,                     //cold_milk
            C,                     //cocoa
            M,                     //coffee
            E,                     //foam
            A,                     //water
            0,                     //orders_served
        );
    }

    #[test]
    fn test_take_orders_one_invalid_order() {
        assert_all_stats(
            "src/tests/one_invalid_order.txt", // path
            0,                                 // grains_used
            0,                                 // cold_milk_used
            0,                                 //cocoa_used
            0,                                 //coffee_used
            0,                                 //foam_used
            0,                                 //water_used
            G,                                 //grains
            L,                                 //cold_milk
            C,                                 //cocoa
            M,                                 //coffee
            E,                                 //foam
            A,                                 //water
            0,                                 //orders_served
        );
    }

    #[test]
    fn test_take_orders_multiple_orders_one_invalid() {
        assert_all_stats(
            "src/tests/multiple_orders_one_invalid.txt", // path
            0,                                           // grains_used
            0,                                           // cold_milk_used
            28,                                          //cocoa_used
            24,                                          //coffee_used
            30,                                          //foam_used
            26,                                          //water_used
            G,                                           //grains
            L,                                           //cold_milk
            C - 28,                                      //cocoa
            M - 24,                                      //coffee
            E - 30,                                      //foam
            A - 26,                                      //water
            4,                                           //orders_served
        );
    }
}
