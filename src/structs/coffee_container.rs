use std::cmp::min;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

use crate::helpers::constants::{CANTIDAD_RELLENO, G, M, REFILL_COFFEE_TIME, X};
use crate::helpers::error::CustomError;

/// Contenedor de café.
pub struct CoffeeContainer {
    /// Cantidad de granos de café disponible para moler y hacer cafe molido.
    pub coffee_grains_container: u32,
    /// Cantidad de café molido disponible para el uso.
    pub ground_coffee_container: u32,
    /// Cantidad de granos de café ya utilizados.
    pub coffee_grains_used: u32,
    /// Cantidad de café molido ya utilizado.
    pub ground_coffee_used: u32,
    /// Flag para indicar que ya no se deben rellenar el café molido.
    pub shutdown: bool,
}

impl CoffeeContainer {
    pub fn new() -> CoffeeContainer {
        CoffeeContainer {
            coffee_grains_container: G,
            ground_coffee_container: M,
            coffee_grains_used: 0,
            ground_coffee_used: 0,
            shutdown: false,
        }
    }

    /// Rellena el café molido cuando tiene una disponibilidad menor a [`CANTIDAD_RELLENO`].
    /// Es un loop donde se tiene en cuenta la disponibilidad del café molido y si el mismo debe apagarse.
    /// Mientras se esta recargando el cafe molido no se puede utilizar el contenedor.
    /// Si la cantidad de granos llega a cero, se deja de ejecutar ya que no se pueden recargar los granos.
    /// Al llegar al [`X%`] de su disponibilidad de granos se alerta por pantalla.
    pub fn grind_coffee(
        coffee_container: Arc<(Mutex<CoffeeContainer>, Condvar)>,
    ) -> Result<(), CustomError> {
        let (coffee_lock, coffee_cvar) = &*coffee_container;
        loop {
            if let Ok(mut state) = coffee_cvar.wait_while(coffee_lock.lock()?, |coffee_container| {
                coffee_container.ground_coffee_container > CANTIDAD_RELLENO
                    && !coffee_container.shutdown
            }) {
                if state.shutdown {
                    break;
                }
                if state.coffee_grains_container == 0 {
                    println!("[DEBUG] No hay mas granos.");
                    break;
                }
                println!("[DEBUG] Rellenando el café molido.");
                thread::sleep(Duration::from_millis(REFILL_COFFEE_TIME));
                let grains_to_grind = min(
                    M - state.ground_coffee_container,
                    state.coffee_grains_container,
                );
                state.ground_coffee_container += grains_to_grind;
                state.coffee_grains_container -= grains_to_grind;
                state.coffee_grains_used += grains_to_grind;
                let capacity_percentage = X as f32 / 100.0 * G as f32;
                if (state.coffee_grains_container as f32) < capacity_percentage {
                    println!("[WARN] El contenedor de granos se encuentra por debajo de {:?}% de su capacidad", X);
                }
                coffee_cvar.notify_all();
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_coffee_container() {
        let coffee_container = CoffeeContainer::new();
        assert_eq!(coffee_container.coffee_grains_container, G);
        assert_eq!(coffee_container.ground_coffee_container, M);
        assert_eq!(coffee_container.coffee_grains_used, 0);
        assert_eq!(coffee_container.ground_coffee_used, 0);
        assert_eq!(coffee_container.shutdown, false);
    }

    #[test]
    fn test_grind_coffee_refill() -> Result<(), CustomError> {
        let coffee_container = Arc::new((Mutex::new(CoffeeContainer::new()), Condvar::new()));
        let coffee_container_clone = coffee_container.clone();
        let thread_handle =
            thread::spawn(
                move || match CoffeeContainer::grind_coffee(coffee_container_clone) {
                    Ok(_) => {}
                    Err(e) => {
                        println!("[ERROR] Testeando grind_coffee: {:?}", e);
                    }
                },
            );

        let (coffee_lock, coffee_cvar) = &*coffee_container;
        match coffee_lock.lock() {
            Ok(mut coffee_lock) => {
                assert_eq!(coffee_lock.coffee_grains_used, 0);
                assert_eq!(coffee_lock.ground_coffee_used, 0);
                assert_eq!(coffee_lock.coffee_grains_container, G);
                assert_eq!(coffee_lock.ground_coffee_container, M);

                coffee_lock.ground_coffee_container = CANTIDAD_RELLENO - 1;
                coffee_lock.ground_coffee_used = M - CANTIDAD_RELLENO + 1;
            }
            Err(e) => {
                println!("[ERROR] Testeando grind_coffee: {:?}", e);
            }
        }
        coffee_cvar.notify_all();

        let (coffee_lock, coffee_cvar) = &*coffee_container;
        if let Ok(state) = coffee_cvar.wait_while(coffee_lock.lock()?, |coffee_container| {
            coffee_container.coffee_grains_container == G
        }) {
            assert_eq!(state.coffee_grains_used, M - CANTIDAD_RELLENO + 1);
            assert_eq!(state.ground_coffee_used, M - CANTIDAD_RELLENO + 1);
            assert_eq!(state.coffee_grains_container, G - M + CANTIDAD_RELLENO - 1);
            assert_eq!(state.ground_coffee_container, M);
        }
        coffee_cvar.notify_all();

        let (coffee_lock, coffee_cvar) = &*coffee_container;
        match coffee_lock.lock() {
            Ok(mut coffee_lock) => {
                coffee_lock.shutdown = true;
            }
            Err(e) => {
                println!("[ERROR] Testeando grind_coffee: {:?}", e);
                return Err(CustomError::TestFailing);
            }
        }
        coffee_cvar.notify_all();

        if thread_handle.join().is_err() {
            println!("[ERROR] Testeando grind_coffee, uniendo hilo.");
            return Err(CustomError::TestFailing);
        }
        Ok(())
    }

    #[test]
    fn test_grind_coffee_grains_empty_wont_refill() -> Result<(), CustomError> {
        let coffee_container = Arc::new((
            Mutex::new(CoffeeContainer {
                coffee_grains_container: 0,
                ground_coffee_container: 40,
                coffee_grains_used: 0,
                ground_coffee_used: 0,
                shutdown: false,
            }),
            Condvar::new(),
        ));
        let coffee_container_clone = coffee_container.clone();
        let thread_handle =
            thread::spawn(
                move || match CoffeeContainer::grind_coffee(coffee_container_clone) {
                    Ok(_) => {}
                    Err(e) => {
                        println!("[ERROR] Testeando grind_coffee: {:?}", e);
                    }
                },
            );

        let (coffee_lock, coffee_cvar) = &*coffee_container;
        match coffee_lock.lock() {
            Ok(mut coffee_lock) => {
                assert_eq!(coffee_lock.coffee_grains_container, 0);
                assert_eq!(coffee_lock.ground_coffee_container, 40);

                coffee_lock.ground_coffee_container = 15;
            }
            Err(e) => {
                println!("[ERROR] Testeando grind_coffee: {:?}", e);
            }
        }
        coffee_cvar.notify_all();

        let (coffee_lock, coffee_cvar) = &*coffee_container;
        if let Ok(state) = coffee_cvar.wait_while(coffee_lock.lock()?, |coffee_container| {
            coffee_container.coffee_grains_container == G
        }) {
            assert_eq!(state.coffee_grains_container, 0);
            assert_eq!(state.ground_coffee_container, 15);
        }
        coffee_cvar.notify_all();

        let (coffee_lock, coffee_cvar) = &*coffee_container;
        match coffee_lock.lock() {
            Ok(mut coffee_lock) => {
                coffee_lock.shutdown = true;
            }
            Err(e) => {
                println!("[ERROR] Testeando grind_coffee: {:?}", e);
                return Err(CustomError::TestFailing);
            }
        }
        coffee_cvar.notify_all();

        if thread_handle.join().is_err() {
            println!("[ERROR] Testeando grind_coffee, uniendo hilo.");
            return Err(CustomError::TestFailing);
        }
        Ok(())
    }

    #[test]
    fn test_grind_coffee_shutdown() -> Result<(), CustomError> {
        let coffee_container = Arc::new((Mutex::new(CoffeeContainer::new()), Condvar::new()));
        let coffee_container_clone = coffee_container.clone();
        let thread_handle =
            thread::spawn(
                move || match CoffeeContainer::grind_coffee(coffee_container_clone) {
                    Ok(_) => {}
                    Err(e) => {
                        println!("[ERROR] Testeando grind_coffee: {:?}", e);
                    }
                },
            );

        let (coffee_lock, coffee_cvar) = &*coffee_container;
        match coffee_lock.lock() {
            Ok(mut coffee_lock) => {
                coffee_lock.shutdown = true;
            }
            Err(e) => {
                println!("[ERROR] Testeando grind_coffee: {:?}", e);
                return Err(CustomError::TestFailing);
            }
        }
        coffee_cvar.notify_all();

        if thread_handle.join().is_err() {
            println!("[ERROR] Testeando grind_coffee, uniendo hilo.");
            return Err(CustomError::TestFailing);
        }

        let (coffee_lock, coffee_cvar) = &*coffee_container;
        match coffee_lock.lock() {
            Ok(coffee_lock) => {
                assert_eq!(coffee_lock.shutdown, true);
            }
            Err(e) => {
                println!("[ERROR] Testeando grind_coffee: {:?}", e);
            }
        }
        coffee_cvar.notify_all();

        Ok(())
    }
}
