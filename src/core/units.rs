//! Erbium Currency Units
//!
//! Sistema de unidades monetárias da blockchain Erbium, similar ao Bitcoin (satoshi)
//! e Ethereum (wei). Baseado em 8 casas decimais para alta precisão.
//!
//! # Estrutura das Unidades
//!
//! | Unidade      | Símbolo | Valor em ERB    | Fator de Conversão | Observação |
//! |--------------|---------|-----------------|-------------------|------------|
//! | Ion          | ion     | 0.00000001 ERB  | 10^-8             | Unidade mínima |
//! | MicroERB     | μERB    | 0.000001 ERB    | 10^-6             | 100 ions |
//! | MilliERB     | mERB    | 0.001 ERB       | 10^-3             | Milésimo |
//! | CentiERB     | cERB    | 0.01 ERB        | 10^-2             | Centésimo |
//! | DeciERB     | dERB    | 0.1 ERB         | 10^-1             | Décimo |
//! | ERB          | ERB     | 1.0 ERB         | 10^0              | Unidade principal |
//! | KiloERB      | kERB    | 1,000 ERB       | 10^3              | Mil ERB |
//! | MegaERB      | M-ERB   | 1,000,000 ERB   | 10^6              | Milhão de ERB |
//!
//! # Exemplo de Uso
//!
//! ```rust
//! use erbium_blockchain::core::units::{ErbAmount, Unit};
//!
//! // Criar valor em ERB
//! let amount = ErbAmount::from_erb(1.5);
//!
//! // Converter para ions
//! let ions = amount.to_ions();
//! assert_eq!(ions, 150_000_000);
//!
//! // Converter para string formatada
//! assert_eq!(amount.format(Unit::ERB), "1.5 ERB");
//! assert_eq!(amount.format(Unit::Ion), "150000000 ion");
//! ```

use std::fmt;

/// Representa uma quantidade de ERB com alta precisão
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ErbAmount {
    /// Valor em ions (unidade mínima, 10^-8 ERB)
    pub ions: u128,
}

impl ErbAmount {
    /// Número de ions em 1 ERB
    pub const IONS_PER_ERB: u128 = 100_000_000;

    /// Máximo supply em ERB (1 bilhão)
    pub const MAX_SUPPLY_ERB: u128 = 1_000_000_000;

    /// Máximo supply em ions
    pub const MAX_SUPPLY_IONS: u128 = Self::MAX_SUPPLY_ERB * Self::IONS_PER_ERB;

    /// Criar ErbAmount a partir de valor em ERB (f64)
    pub fn from_erb(erb: f64) -> Self {
        let ions = (erb * Self::IONS_PER_ERB as f64) as u128;
        Self { ions }
    }

    /// Criar ErbAmount a partir de ions
    pub fn from_ions(ions: u128) -> Self {
        Self { ions }
    }

    /// Criar ErbAmount a partir de string (ex: "1.5 ERB", "150000000 ion")
    pub fn from_string(s: &str) -> Result<Self, String> {
        let s = s.trim();

        // Tentar identificar a unidade
        if s.ends_with(" ion") || s.ends_with(" ions") {
            let value_str = s.trim_end_matches(" ion").trim_end_matches(" ions");
            let ions = value_str
                .parse::<u128>()
                .map_err(|_| format!("Valor inválido para ions: {}", value_str))?;
            Ok(Self::from_ions(ions))
        } else if s.ends_with(" μERB") || s.ends_with(" microERB") {
            let value_str = s.trim_end_matches(" μERB").trim_end_matches(" microERB");
            let micro_erb = value_str
                .parse::<f64>()
                .map_err(|_| format!("Valor inválido para microERB: {}", value_str))?;
            Ok(Self::from_erb(micro_erb / 1_000_000.0))
        } else if s.ends_with(" mERB") || s.ends_with(" milliERB") {
            let value_str = s.trim_end_matches(" mERB").trim_end_matches(" milliERB");
            let milli_erb = value_str
                .parse::<f64>()
                .map_err(|_| format!("Valor inválido para milliERB: {}", value_str))?;
            Ok(Self::from_erb(milli_erb / 1_000.0))
        } else if s.ends_with(" cERB") || s.ends_with(" centiERB") {
            let value_str = s.trim_end_matches(" cERB").trim_end_matches(" centiERB");
            let centi_erb = value_str
                .parse::<f64>()
                .map_err(|_| format!("Valor inválido para centiERB: {}", value_str))?;
            Ok(Self::from_erb(centi_erb / 100.0))
        } else if s.ends_with(" dERB") || s.ends_with(" deciERB") {
            let value_str = s.trim_end_matches(" dERB").trim_end_matches(" deciERB");
            let deci_erb = value_str
                .parse::<f64>()
                .map_err(|_| format!("Valor inválido para deciERB: {}", value_str))?;
            Ok(Self::from_erb(deci_erb / 10.0))
        } else if s.ends_with(" ERB") {
            let value_str = s.trim_end_matches(" ERB");
            let erb = value_str
                .parse::<f64>()
                .map_err(|_| format!("Valor inválido para ERB: {}", value_str))?;
            Ok(Self::from_erb(erb))
        } else if s.ends_with(" kERB") || s.ends_with(" kiloERB") {
            let value_str = s.trim_end_matches(" kERB").trim_end_matches(" kiloERB");
            let kilo_erb = value_str
                .parse::<f64>()
                .map_err(|_| format!("Valor inválido para kiloERB: {}", value_str))?;
            Ok(Self::from_erb(kilo_erb * 1_000.0))
        } else if s.ends_with(" M-ERB") || s.ends_with(" megaERB") {
            let value_str = s.trim_end_matches(" M-ERB").trim_end_matches(" megaERB");
            let mega_erb = value_str
                .parse::<f64>()
                .map_err(|_| format!("Valor inválido para megaERB: {}", value_str))?;
            Ok(Self::from_erb(mega_erb * 1_000_000.0))
        } else {
            // Assume ERB se não especificado
            let erb = s
                .parse::<f64>()
                .map_err(|_| format!("Valor inválido: {}", s))?;
            Ok(Self::from_erb(erb))
        }
    }

    /// Converter para valor em ERB (f64)
    pub fn to_erb(&self) -> f64 {
        self.ions as f64 / Self::IONS_PER_ERB as f64
    }

    /// Obter valor em ions
    pub fn to_ions(&self) -> u128 {
        self.ions
    }

    /// Formatar valor com unidade específica
    pub fn format(&self, unit: Unit) -> String {
        match unit {
            Unit::Ion => format!("{} ion", self.ions),
            Unit::MicroERB => {
                let micro_erb = self.to_erb() * 1_000_000.0;
                format!("{:.6} μERB", micro_erb)
            }
            Unit::MilliERB => {
                let milli_erb = self.to_erb() * 1_000.0;
                format!("{:.3} mERB", milli_erb)
            }
            Unit::CentiERB => {
                let centi_erb = self.to_erb() * 100.0;
                format!("{:.2} cERB", centi_erb)
            }
            Unit::DeciERB => {
                let deci_erb = self.to_erb() * 10.0;
                format!("{:.1} dERB", deci_erb)
            }
            Unit::ERB => {
                let erb = self.to_erb();
                if erb.fract() == 0.0 {
                    format!("{:.0} ERB", erb)
                } else {
                    // Remove trailing zeros and dot if present
                    let formatted = format!("{:.8}", erb)
                        .trim_end_matches('0')
                        .trim_end_matches('.')
                        .to_string();
                    format!("{} ERB", formatted)
                }
            }
            Unit::KiloERB => {
                let kilo_erb = self.to_erb() / 1_000.0;
                format!("{:.3} kERB", kilo_erb)
            }
            Unit::MegaERB => {
                let mega_erb = self.to_erb() / 1_000_000.0;
                format!("{:.3} M-ERB", mega_erb)
            }
        }
    }

    /// Formatar automaticamente com a melhor unidade
    pub fn format_auto(&self) -> String {
        let erb = self.to_erb();

        if erb >= 1_000_000.0 {
            self.format(Unit::MegaERB)
        } else if erb >= 1_000.0 {
            self.format(Unit::KiloERB)
        } else if erb >= 1.0 {
            self.format(Unit::ERB)
        } else if erb >= 0.1 {
            self.format(Unit::DeciERB)
        } else if erb >= 0.01 {
            self.format(Unit::CentiERB)
        } else if erb >= 0.001 {
            self.format(Unit::MilliERB)
        } else if erb >= 0.000_001 {
            self.format(Unit::MicroERB)
        } else {
            self.format(Unit::Ion)
        }
    }

    /// Operações aritméticas
    pub fn add(&self, other: &ErbAmount) -> Result<Self, String> {
        let result = self
            .ions
            .checked_add(other.ions)
            .ok_or("Overflow na adição")?;
        Ok(Self { ions: result })
    }

    pub fn sub(&self, other: &ErbAmount) -> Result<Self, String> {
        let result = self
            .ions
            .checked_sub(other.ions)
            .ok_or("Underflow na subtração")?;
        Ok(Self { ions: result })
    }

    pub fn mul(&self, factor: u128) -> Result<Self, String> {
        let result = self
            .ions
            .checked_mul(factor)
            .ok_or("Overflow na multiplicação")?;
        Ok(Self { ions: result })
    }

    pub fn div(&self, divisor: u128) -> Result<Self, String> {
        if divisor == 0 {
            return Err("Divisão por zero".to_string());
        }
        Ok(Self {
            ions: self.ions / divisor,
        })
    }
}

/// Unidades monetárias disponíveis
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    Ion,
    MicroERB,
    MilliERB,
    CentiERB,
    DeciERB,
    ERB,
    KiloERB,
    MegaERB,
}

impl fmt::Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Unit::Ion => write!(f, "ion"),
            Unit::MicroERB => write!(f, "μERB"),
            Unit::MilliERB => write!(f, "mERB"),
            Unit::CentiERB => write!(f, "cERB"),
            Unit::DeciERB => write!(f, "dERB"),
            Unit::ERB => write!(f, "ERB"),
            Unit::KiloERB => write!(f, "kERB"),
            Unit::MegaERB => write!(f, "M-ERB"),
        }
    }
}

impl Unit {
    /// Fator de conversão para ERB
    pub fn to_erb_factor(&self) -> f64 {
        match self {
            Unit::Ion => 1.0 / ErbAmount::IONS_PER_ERB as f64,
            Unit::MicroERB => 0.000_001,
            Unit::MilliERB => 0.001,
            Unit::CentiERB => 0.01,
            Unit::DeciERB => 0.1,
            Unit::ERB => 1.0,
            Unit::KiloERB => 1_000.0,
            Unit::MegaERB => 1_000_000.0,
        }
    }

    /// Fator de conversão para ions
    pub fn to_ions_factor(&self) -> u128 {
        match self {
            Unit::Ion => 1,
            Unit::MicroERB => ErbAmount::IONS_PER_ERB / 1_000_000,
            Unit::MilliERB => ErbAmount::IONS_PER_ERB / 1_000,
            Unit::CentiERB => ErbAmount::IONS_PER_ERB / 100,
            Unit::DeciERB => ErbAmount::IONS_PER_ERB / 10,
            Unit::ERB => ErbAmount::IONS_PER_ERB,
            Unit::KiloERB => ErbAmount::IONS_PER_ERB * 1_000,
            Unit::MegaERB => ErbAmount::IONS_PER_ERB * 1_000_000,
        }
    }
}

/// Constantes para valores comuns
pub mod constants {
    use super::ErbAmount;

    pub const ONE_ION: ErbAmount = ErbAmount { ions: 1 };
    pub const ONE_MICRO_ERB: ErbAmount = ErbAmount { ions: 100 }; // 10^-6 ERB
    pub const ONE_MILLI_ERB: ErbAmount = ErbAmount { ions: 100_000 }; // 10^-3 ERB
    pub const ONE_CENTI_ERB: ErbAmount = ErbAmount { ions: 1_000_000 }; // 10^-2 ERB
    pub const ONE_DECI_ERB: ErbAmount = ErbAmount { ions: 10_000_000 }; // 10^-1 ERB
    pub const ONE_ERB: ErbAmount = ErbAmount { ions: 100_000_000 }; // 1 ERB
    pub const ONE_KILO_ERB: ErbAmount = ErbAmount {
        ions: 100_000_000_000,
    }; // 10^3 ERB
    pub const ONE_MEGA_ERB: ErbAmount = ErbAmount {
        ions: 100_000_000_000_000,
    }; // 10^6 ERB

    // Reserva pessoal (50M ERB)
    pub const PERSONAL_RESERVE: ErbAmount = ErbAmount {
        ions: 50_000_000 * 100_000_000,
    };

    // Development Fund (20M ERB)
    pub const DEV_FUND: ErbAmount = ErbAmount {
        ions: 20_000_000 * 100_000_000,
    };

    // Ecosystem Fund (20M ERB)
    pub const ECO_FUND: ErbAmount = ErbAmount {
        ions: 20_000_000 * 100_000_000,
    };

    // Treasury (10M ERB)
    pub const TREASURY: ErbAmount = ErbAmount {
        ions: 10_000_000 * 100_000_000,
    };

    // Total pré-alocado (100M ERB)
    pub const TOTAL_PREALLOCATED: ErbAmount = ErbAmount {
        ions: 100_000_000 * 100_000_000,
    };

    // Supply inicial total (1B ERB)
    pub const INITIAL_TOTAL_SUPPLY: ErbAmount = ErbAmount {
        ions: 1_000_000_000 * 100_000_000,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_conversions() {
        let amount = ErbAmount::from_erb(1.0);
        assert_eq!(amount.to_ions(), 100_000_000);
        assert_eq!(amount.to_erb(), 1.0);
    }

    #[test]
    fn test_string_parsing() {
        assert_eq!(
            ErbAmount::from_string("1 ERB").unwrap().to_ions(),
            100_000_000
        );
        assert_eq!(
            ErbAmount::from_string("100000000 ion").unwrap().to_ions(),
            100_000_000
        );
        assert_eq!(ErbAmount::from_string("1 μERB").unwrap().to_ions(), 100);
    }

    #[test]
    fn test_formatting() {
        let amount = ErbAmount::from_erb(1.5);
        assert_eq!(amount.format(Unit::ERB), "1.50000000 ERB");
        assert_eq!(amount.format(Unit::Ion), "150000000 ion");
    }

    #[test]
    fn test_auto_formatting() {
        assert_eq!(
            ErbAmount::from_erb(1_500_000.0).format_auto(),
            "1.500 M-ERB"
        );
        assert_eq!(ErbAmount::from_erb(1.5).format_auto(), "1.50000000 ERB");
        assert_eq!(ErbAmount::from_ions(150).format_auto(), "1.500000 μERB");
        assert_eq!(ErbAmount::from_ions(1).format_auto(), "1 ion");
    }

    #[test]
    fn test_constants() {
        assert_eq!(constants::ONE_ERB.to_ions(), 100_000_000);
        assert_eq!(constants::PERSONAL_RESERVE.to_erb(), 50_000_000.0);
        assert_eq!(constants::TOTAL_PREALLOCATED.to_erb(), 100_000_000.0);
    }
}
