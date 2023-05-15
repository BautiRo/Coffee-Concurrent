use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

use crate::helpers::constants::{A, CANTIDAD_RELLENO, REFILL_WATER_TIME};
use crate::helpers::error::CustomError;

/// Contenedor de agua caliente conectado a la red.
#[derive(Debug)]
pub struct HotWaterContainer {
    /// Cantidad de agua caliente disponible para su uso.
    pub hot_water: u32,
    /// Cantidad de agua caliente ya utilizada.
    pub used: u32,
    /// Flag para indicar que ya no se deben rellenar el agua caliente.
    pub shutdown: bool,
}

/// Caliente agua de la red cuando tiene una disponibilidad menor a [`CANTIDAD_RELLENO`].
/// Es un loop donde se tiene en cuenta la disponibilidad del agua caliente y si el mismo debe apagarse.
/// Mientras se esta recargando el agua caliente no se puede utilizar el contenedor.
/// Solo termina cuando debe apagarse. Como esta conectada a la red podemos suponer que nunca se quedará sin agua.
impl HotWaterContainer {
    pub fn new() -> HotWaterContainer {
        HotWaterContainer {
            hot_water: A,
            used: 0,
            shutdown: false,
        }
    }

    pub fn heat_water(
        hot_water_container: Arc<(Mutex<HotWaterContainer>, Condvar)>,
    ) -> Result<(), CustomError> {
        let (h_w_lock, h_w_cvar) = &*hot_water_container;
        loop {
            if let Ok(mut state) = h_w_cvar.wait_while(h_w_lock.lock()?, |h_w_container| {
                h_w_container.hot_water > CANTIDAD_RELLENO && !h_w_container.shutdown
            }) {
                if state.shutdown {
                    break;
                }
                println!("[DEBUG] Calentando agua.");
                thread::sleep(Duration::from_millis(REFILL_WATER_TIME));
                state.hot_water = A;
                h_w_cvar.notify_all();
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_hot_water_container() {
        let hot_water_container = HotWaterContainer::new();
        assert_eq!(hot_water_container.hot_water, A);
        assert_eq!(hot_water_container.used, 0);
        assert_eq!(hot_water_container.shutdown, false);
    }

    #[test]
    fn test_heat_water_refill() -> Result<(), CustomError> {
        let hot_water_container = Arc::new((Mutex::new(HotWaterContainer::new()), Condvar::new()));
        let hot_water_container_clone = hot_water_container.clone();
        let thread_handle =
            thread::spawn(
                move || match HotWaterContainer::heat_water(hot_water_container_clone) {
                    Ok(_) => {}
                    Err(e) => {
                        println!("[ERROR] Testeando heat_water: {:?}", e);
                    }
                },
            );

        let (hot_water_lock, hot_water_cvar) = &*hot_water_container;
        match hot_water_lock.lock() {
            Ok(mut hot_water_lock) => {
                assert_eq!(hot_water_lock.used, 0);
                assert_eq!(hot_water_lock.hot_water, A);

                hot_water_lock.hot_water = CANTIDAD_RELLENO - 1;
                hot_water_lock.used = A - CANTIDAD_RELLENO + 1;
            }
            Err(e) => {
                println!("[ERROR] Testeando heat_water: {:?}", e);
            }
        }
        hot_water_cvar.notify_all();

        let (hot_water_lock, hot_water_cvar) = &*hot_water_container;
        if let Ok(state) = hot_water_cvar
            .wait_while(hot_water_lock.lock()?, |hot_water_container| {
                hot_water_container.hot_water < A
            })
        {
            assert_eq!(state.used, A - CANTIDAD_RELLENO + 1);
            assert_eq!(state.hot_water, A);
        }
        hot_water_cvar.notify_all();

        let (hot_water_lock, hot_water_cvar) = &*hot_water_container;
        match hot_water_lock.lock() {
            Ok(mut hot_water_lock) => {
                hot_water_lock.shutdown = true;
            }
            Err(e) => {
                println!("[ERROR] Testeando heat_water: {:?}", e);
                return Err(CustomError::TestFailing);
            }
        }
        hot_water_cvar.notify_all();

        if thread_handle.join().is_err() {
            println!("[ERROR] Testeando heat_water, uniendo hilo.");
            return Err(CustomError::TestFailing);
        }
        Ok(())
    }

    #[test]
    fn test_heat_water_shutdown() -> Result<(), CustomError> {
        let hot_water_container = Arc::new((Mutex::new(HotWaterContainer::new()), Condvar::new()));
        let hot_water_container_clone = hot_water_container.clone();
        let thread_handle =
            thread::spawn(
                move || match HotWaterContainer::heat_water(hot_water_container_clone) {
                    Ok(_) => {}
                    Err(e) => {
                        println!("[ERROR] Testeando heat_water: {:?}", e);
                    }
                },
            );

        let (hot_water_lock, hot_water_cvar) = &*hot_water_container;
        match hot_water_lock.lock() {
            Ok(mut hot_water_lock) => {
                hot_water_lock.shutdown = true;
            }
            Err(e) => {
                println!("[ERROR] Testeando heat_water: {:?}", e);
                return Err(CustomError::TestFailing);
            }
        }
        hot_water_cvar.notify_all();

        if thread_handle.join().is_err() {
            println!("[ERROR] Testeando heat_water, uniendo hilo.");
            return Err(CustomError::TestFailing);
        }

        let (hot_water_lock, hot_water_cvar) = &*hot_water_container;
        match hot_water_lock.lock() {
            Ok(hot_water_lock) => {
                assert_eq!(hot_water_lock.shutdown, true);
            }
            Err(e) => {
                println!("[ERROR] Testeando heat_water: {:?}", e);
            }
        }
        hot_water_cvar.notify_all();

        Ok(())
    }
}
