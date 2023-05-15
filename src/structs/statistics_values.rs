/// Estructura utilizada únicamente para las estadísticas.
pub struct StatisticsValues {
    /// Contador de las ordenes que ya fueron completadas.
    pub orders_served: u32,
    /// Flag utilizado para apagar el hilo que imprime las estadísticas.
    pub shutdown: bool,
}

impl StatisticsValues {
    pub fn new() -> StatisticsValues {
        StatisticsValues {
            orders_served: 0,
            shutdown: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_statistics_values() {
        let statistics_values = StatisticsValues::new();
        assert_eq!(statistics_values.orders_served, 0);
        assert_eq!(statistics_values.shutdown, false);
    }
}
