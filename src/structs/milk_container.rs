use std::cmp::min;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

use crate::helpers::constants::{CANTIDAD_RELLENO, E, L, REFILL_MILK_TIME, X};
use crate::helpers::error::CustomError;

/// Contenedor de leche.
pub struct MilkContainer {
    /// Cantidad de leche fría disponible para calentarse y hacer espuma.
    pub cold_milk_container: u32,
    /// Cantidad de espuma de leche disponible para su uso.
    pub milk_foam_container: u32,
    /// Cantidad de leche fría ya utilizada.
    pub cold_milk_used: u32,
    /// Cantidad de espuma de leche ya utilizada.
    pub milk_foam_used: u32,
    /// Flag para indicar que ya no se deben rellenar la espuma de leche.
    pub shutdown: bool,
}

impl MilkContainer {
    pub fn new() -> MilkContainer {
        MilkContainer {
            cold_milk_container: L,
            milk_foam_container: E,
            cold_milk_used: 0,
            milk_foam_used: 0,
            shutdown: false,
        }
    }

    /// Rellena el cafe molido cuando tiene una disponibilidad menor a [`CANTIDAD_RELLENO`].
    /// Es un loop donde se tiene en cuenta la disponibilidad de la espuma de leche y si el mismo debe apagarse.
    /// Mientras se esta recargando la espuma de leche no se puede utilizar el contenedor.
    /// Si la cantidad de leche fría llega a cero, se deja de ejecutar ya que no se puede recargar.
    /// Al llegar al [`X%`] de su disponibilidad de leche fría se alerta por pantalla.
    pub fn make_milk_foam(
        milk_container: Arc<(Mutex<MilkContainer>, Condvar)>,
    ) -> Result<(), CustomError> {
        let (milk_lock, milk_cvar) = &*milk_container;
        loop {
            if let Ok(mut state) = milk_cvar.wait_while(milk_lock.lock()?, |milk_container| {
                milk_container.milk_foam_container > CANTIDAD_RELLENO && !milk_container.shutdown
            }) {
                if state.shutdown {
                    break;
                }
                if state.cold_milk_container == 0 {
                    println!("[INFO] No hay mas leche fria.");
                    break;
                }
                println!("[DEBUG] Rellenando espuma de leche.");
                thread::sleep(Duration::from_millis(REFILL_MILK_TIME));
                let milk_to_foam = min(E - state.milk_foam_container, state.cold_milk_container);
                state.milk_foam_container += milk_to_foam;
                state.cold_milk_container -= milk_to_foam;
                state.cold_milk_used += milk_to_foam;
                let capacity_percentage = X as f32 / 100.0 * L as f32;
                if (state.cold_milk_container as f32) < capacity_percentage {
                    println!("[WARN] El contenedor de leche se encuentra por debajo de {:?}% de su capacidad", X);
                }
                milk_cvar.notify_all();
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_milk_container() {
        let milk_container = MilkContainer::new();
        assert_eq!(milk_container.cold_milk_container, L);
        assert_eq!(milk_container.milk_foam_container, E);
        assert_eq!(milk_container.cold_milk_used, 0);
        assert_eq!(milk_container.milk_foam_used, 0);
        assert_eq!(milk_container.shutdown, false);
    }

    #[test]
    fn test_make_milk_foam_refill() -> Result<(), CustomError> {
        let milk_container = Arc::new((Mutex::new(MilkContainer::new()), Condvar::new()));
        let milk_container_clone = milk_container.clone();
        let thread_handle =
            thread::spawn(
                move || match MilkContainer::make_milk_foam(milk_container_clone) {
                    Ok(_) => {}
                    Err(e) => {
                        println!("[ERROR] Testeando make_milk_foam: {:?}", e);
                    }
                },
            );

        let (milk_lock, milk_cvar) = &*milk_container;
        match milk_lock.lock() {
            Ok(mut milk_lock) => {
                assert_eq!(milk_lock.cold_milk_used, 0);
                assert_eq!(milk_lock.milk_foam_used, 0);
                assert_eq!(milk_lock.cold_milk_container, L);
                assert_eq!(milk_lock.milk_foam_container, E);

                milk_lock.milk_foam_container = CANTIDAD_RELLENO - 1;
                milk_lock.milk_foam_used = E - CANTIDAD_RELLENO + 1;
            }
            Err(e) => {
                println!("[ERROR] Testeando make_milk_foam: {:?}", e);
            }
        }
        milk_cvar.notify_all();

        let (milk_lock, milk_cvar) = &*milk_container;
        if let Ok(state) = milk_cvar.wait_while(milk_lock.lock()?, |milk_container| {
            milk_container.cold_milk_container == L
        }) {
            assert_eq!(state.cold_milk_used, E - CANTIDAD_RELLENO + 1);
            assert_eq!(state.milk_foam_used, E - CANTIDAD_RELLENO + 1);
            assert_eq!(state.cold_milk_container, L - E + CANTIDAD_RELLENO - 1);
            assert_eq!(state.milk_foam_container, E);
        }
        milk_cvar.notify_all();

        let (milk_lock, milk_cvar) = &*milk_container;
        match milk_lock.lock() {
            Ok(mut milk_lock) => {
                milk_lock.shutdown = true;
            }
            Err(e) => {
                println!("[ERROR] Testeando make_milk_foam: {:?}", e);
                return Err(CustomError::TestFailing);
            }
        }
        milk_cvar.notify_all();

        if thread_handle.join().is_err() {
            println!("[ERROR] Testeando make_milk_foam, uniendo hilo.");
            return Err(CustomError::TestFailing);
        }
        Ok(())
    }

    #[test]
    fn test_make_milk_foam_cold_milk_empty_wont_refill() -> Result<(), CustomError> {
        let milk_container = Arc::new((
            Mutex::new(MilkContainer {
                cold_milk_container: 0,
                milk_foam_container: 40,
                cold_milk_used: 0,
                milk_foam_used: 0,
                shutdown: false,
            }),
            Condvar::new(),
        ));
        let milk_container_clone = milk_container.clone();
        let thread_handle =
            thread::spawn(
                move || match MilkContainer::make_milk_foam(milk_container_clone) {
                    Ok(_) => {}
                    Err(e) => {
                        println!("[ERROR] Testeando make_milk_foam: {:?}", e);
                    }
                },
            );

        let (milk_lock, milk_cvar) = &*milk_container;
        match milk_lock.lock() {
            Ok(mut milk_lock) => {
                assert_eq!(milk_lock.cold_milk_container, 0);
                assert_eq!(milk_lock.milk_foam_container, 40);

                milk_lock.milk_foam_container = 15;
            }
            Err(e) => {
                println!("[ERROR] Testeando make_milk_foam: {:?}", e);
            }
        }
        milk_cvar.notify_all();

        let (milk_lock, milk_cvar) = &*milk_container;
        if let Ok(state) = milk_cvar.wait_while(milk_lock.lock()?, |milk_container| {
            milk_container.cold_milk_container == L
        }) {
            assert_eq!(state.cold_milk_container, 0);
            assert_eq!(state.milk_foam_container, 15);
        }
        milk_cvar.notify_all();

        let (milk_lock, milk_cvar) = &*milk_container;
        match milk_lock.lock() {
            Ok(mut milk_lock) => {
                milk_lock.shutdown = true;
            }
            Err(e) => {
                println!("[ERROR] Testeando make_milk_foam: {:?}", e);
                return Err(CustomError::TestFailing);
            }
        }
        milk_cvar.notify_all();

        if thread_handle.join().is_err() {
            println!("[ERROR] Testeando make_milk_foam, uniendo hilo.");
            return Err(CustomError::TestFailing);
        }
        Ok(())
    }

    #[test]
    fn test_make_milk_foam_shutdown() -> Result<(), CustomError> {
        let milk_container = Arc::new((Mutex::new(MilkContainer::new()), Condvar::new()));
        let milk_container_clone = milk_container.clone();
        let thread_handle =
            thread::spawn(
                move || match MilkContainer::make_milk_foam(milk_container_clone) {
                    Ok(_) => {}
                    Err(e) => {
                        println!("[ERROR] Testeando make_milk_foam: {:?}", e);
                    }
                },
            );

        let (milk_lock, milk_cvar) = &*milk_container;
        match milk_lock.lock() {
            Ok(mut milk_lock) => {
                milk_lock.shutdown = true;
            }
            Err(e) => {
                println!("[ERROR] Testeando make_milk_foam: {:?}", e);
                return Err(CustomError::TestFailing);
            }
        }
        milk_cvar.notify_all();

        if thread_handle.join().is_err() {
            println!("[ERROR] Testeando make_milk_foam, uniendo hilo.");
            return Err(CustomError::TestFailing);
        }

        let (milk_lock, milk_cvar) = &*milk_container;
        match milk_lock.lock() {
            Ok(milk_lock) => {
                assert_eq!(milk_lock.shutdown, true);
            }
            Err(e) => {
                println!("[ERROR] Testeando make_milk_foam: {:?}", e);
            }
        }
        milk_cvar.notify_all();

        Ok(())
    }
}
