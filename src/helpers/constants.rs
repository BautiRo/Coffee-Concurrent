/// Capacidad de contenedor de granos de café para moler
pub const G: u32 = 500;
/// Capacidad de contenedor de granos de café molidos
pub const M: u32 = 100;
/// Capacidad de contenedor de leche fría para hacer espuma
pub const L: u32 = 500;
/// Capacidad de contenedor de espuma de leche
pub const E: u32 = 100;
/// Capacidad de contenedor de cacao
pub const C: u32 = 100;
/// Capacidad de contenedor de agua
pub const A: u32 = 100;

/// Capacidad de contenedores de granosde café, leche y cacao donde se debe alertar
pub const X: u32 = 25;
/// Tiempo en milisengudos para que se impriman las estadistincas
pub const TIME_TO_STATS: u64 = 5000;
/// Cantidad de ingredientes en contenedor para que deba ser rellenado.
pub const CANTIDAD_RELLENO: u32 = 30;

/// Tiempo de acción de recibir pedido
pub const TAKE_ORDER_TIME: u64 = 500;
/// Tiempo de acción de servir cafe
pub const SERVE_COFFEE_TIME: u64 = 5000;
/// Tiempo de acción de servir espuma de leche
pub const SERVE_MILK_FOAM_TIME: u64 = 1000;
/// Tiempo de acción de servir agua caliente
pub const SERVE_HOT_WATER_TIME: u64 = 1000;
/// Tiempo de acción de servir cacao
pub const SERVE_COCOA_TIME: u64 = 1000;
/// Tiempo de acción del molinillo automatico de granos y rellenar contenedor
pub const REFILL_COFFEE_TIME: u64 = 1000;
/// Tiempo de acción de calentar leche
pub const REFILL_MILK_TIME: u64 = 1000;
/// Tiempo de acción de tomar agua de red y calentarla
pub const REFILL_WATER_TIME: u64 = 1000;
