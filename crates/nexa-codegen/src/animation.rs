//! Shared physical animation parameters for native backends.

/// Physical spring values corresponding to Nexa's response and damping-ratio inputs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpringPhysics {
    /// Spring stiffness used by Compose and SwiftUI's mass/stiffness spring API.
    pub stiffness: f32,
    /// Viscous damping coefficient for a unit-mass spring.
    pub damping_coefficient: f32,
}

/// Converts Nexa's spring response and damping ratio into shared physical values.
///
/// Compose maps response to stiffness as `100 / response²`. SwiftUI's
/// `interpolatingSpring` accepts physical coefficients, so use the equivalent
/// unit-mass damping coefficient `2ζ√k` to keep both native springs on the same
/// motion curve and settling time.
pub fn spring_physics(response: f32, damping_ratio: f32) -> SpringPhysics {
    let stiffness = (100.0 / f64::from(response).powi(2)) as f32;
    let damping_coefficient = (2.0 * f64::from(damping_ratio) * f64::from(stiffness).sqrt()) as f32;
    SpringPhysics {
        stiffness,
        damping_coefficient,
    }
}

#[cfg(test)]
mod tests {
    use super::spring_physics;

    #[test]
    fn spring_values_match_compose_stiffness_and_equivalent_unit_mass_damping() {
        let physics = spring_physics(0.35, 0.8);

        assert_eq!(physics.stiffness, 816.32654);
        assert!((physics.damping_coefficient - 45.714287).abs() < 0.00001);
    }
}
