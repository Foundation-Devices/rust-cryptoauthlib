mod c2rust;
mod rust2c;
use std::convert::TryFrom;

include!("./types.rs");

// Unfortunately cryptoauthlib takes ATCAIfaceCfg pointer as a field inside
// _gDevice->mIface->mIfaceCFG, so it _must_ be taken from heap.bool
// It adds several lines of code to implement it...
static mut GLOBAL_IFACE_CFG_PTR: *mut cryptoauthlib_sys::ATCAIfaceCfg =
    0 as *mut cryptoauthlib_sys::ATCAIfaceCfg;

/// Creates a global ATCADevice object used by Basic API.
pub fn atcab_init(r_iface_cfg: AtcaIfaceCfg) -> AtcaStatus {
    let mut iface_cfg_ptr;
    let allow_allocation: bool =
        unsafe { GLOBAL_IFACE_CFG_PTR == std::ptr::null_mut::<cryptoauthlib_sys::ATCAIfaceCfg>() };
    if allow_allocation {
        iface_cfg_ptr = Box::new(match rust2c::r2c_atca_iface_cfg(r_iface_cfg) {
            Some(x) => x,
            None => return AtcaStatus::AtcaBadParam,
        });
        unsafe { GLOBAL_IFACE_CFG_PTR = &mut *iface_cfg_ptr };
        std::mem::forget(iface_cfg_ptr);
    }

    c2rust::c2r_enum_status(unsafe { cryptoauthlib_sys::atcab_init(GLOBAL_IFACE_CFG_PTR) })
}

/// Use the SHA command to compute a SHA-256 digest.
pub fn atcab_sha(message: Vec<u8>, digest: &mut Vec<u8>) -> AtcaStatus {
    let length: u16 = match u16::try_from(message.len()) {
        Ok(val) => val,
        Err(_) => return AtcaStatus::AtcaBadParam,
    };

    let digest_size: usize = cryptoauthlib_sys::ATCA_SHA2_256_DIGEST_SIZE as usize;

    if digest.len() != digest_size {
        digest.resize(digest_size, 0);
    }

    c2rust::c2r_enum_status(unsafe {
        cryptoauthlib_sys::atcab_sha(length, message.as_ptr(), digest.as_mut_ptr())
    })
}

/// Get the global device object
pub fn atcab_get_device() -> AtcaDevice {
    AtcaDevice {
        dev: unsafe { cryptoauthlib_sys::atcab_get_device() },
    }
}

pub fn atcab_get_device_type() -> AtcaDeviceType {
    c2rust::c2r_enum_devtype(unsafe { cryptoauthlib_sys::atcab_get_device_type() })
}

pub fn atcab_random(rand_out: &mut Vec<u8>) -> AtcaStatus {
    if rand_out.len() != ACTA_RANDOM_BUFFER_SIZE {
        rand_out.resize(ACTA_RANDOM_BUFFER_SIZE, 0);
    }

    c2rust::c2r_enum_status(unsafe { cryptoauthlib_sys::atcab_random(rand_out.as_mut_ptr()) })
}

fn atcab_is_locked(zone: u8, is_locked: *mut bool) -> AtcaStatus {
    c2rust::c2r_enum_status(unsafe { cryptoauthlib_sys::atcab_is_locked(zone, is_locked) })
}

pub fn atcab_configuration_is_locked(is_locked: &mut bool) -> AtcaStatus {
    atcab_is_locked(ATCA_ZONE_CONFIG, is_locked)
}

fn atcab_get_config_buffer_size() -> usize {
    let device_type = atcab_get_device_type();
    match device_type {
        AtcaDeviceType::ATECC508A | AtcaDeviceType::ATECC608A | AtcaDeviceType::ATECC108A => {
            ATCA_ATECC_CONFIG_BUFFER_SIZE
        }
        _ => ATCA_ATSHA_CONFIG_BUFFER_SIZE,
    }
}

pub fn atcab_read_config_zone(config_data: &mut Vec<u8>) -> AtcaStatus {
    let buffer_size = atcab_get_config_buffer_size();
    if config_data.len() != buffer_size {
        config_data.resize(buffer_size, 0);
    }
    c2rust::c2r_enum_status(unsafe {
        cryptoauthlib_sys::atcab_read_config_zone(config_data.as_mut_ptr())
    })
}

pub fn atcab_cmp_config_zone(config_data: &mut Vec<u8>, same_config: &mut bool) -> AtcaStatus {
    let buffer_size = atcab_get_config_buffer_size();
    if config_data.len() != buffer_size {
        return AtcaStatus::AtcaBadParam;
    }
    c2rust::c2r_enum_status(unsafe {
        cryptoauthlib_sys::atcab_cmp_config_zone(config_data.as_mut_ptr(), same_config)
    })
}

pub fn atcab_release() -> AtcaStatus {
    c2rust::c2r_enum_status(unsafe { cryptoauthlib_sys::atcab_release() })
}

pub fn atca_iface_setup_i2c(
    device_type: String,
    wake_delay: u16,
    rx_retries: i32,
    // I2C salve address
    slave_address: Option<u8>,
    // I2C bus number
    bus: Option<u8>,
    // I2C baud rate
    baud: Option<u32>,
) -> Result<AtcaIfaceCfg, String> {
    let atca_iface_cfg = AtcaIfaceCfg {
        iface_type: AtcaIfaceType::AtcaI2cIface,
        devtype: match device_type.as_str() {
            "atecc608a" => AtcaDeviceType::ATECC608A,
            "atecc508a" => AtcaDeviceType::ATECC508A,
            _ => {
                let e = "Unsupported device type ".to_owned() + device_type.as_str();
                return Err(e);
            }
        },
        iface: AtcaIface {
            atcai2c: AtcaIfaceI2c {
                // unwrap_or_else_return()?
                slave_address: match slave_address {
                    Some(x) => x,
                    _ => return Err("missing i2c slave address".to_owned()),
                },
                bus: match bus {
                    Some(x) => x,
                    _ => return Err("missing i2c bus".to_owned()),
                },
                baud: match baud {
                    Some(x) => x,
                    _ => return Err("missing i2c baud rate".to_owned()),
                },
            },
        },
        rx_retries,
        wake_delay,
    };
    Ok(atca_iface_cfg)
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use serial_test::serial;
    use std::fs::read_to_string;
    use std::path::Path;

    #[derive(Deserialize)]
    struct Config {
        pub device: Device,
        pub interface: Interface,
    }

    #[derive(Deserialize)]
    struct Device {
        pub device_type: String,
        pub iface_type: String,
        pub wake_delay: u16,
        pub rx_retries: i32,
    }

    #[derive(Deserialize)]
    struct Interface {
        pub slave_address: u8,
        pub bus: u8,
        pub baud: u32,
    }

    #[allow(dead_code)]
    fn atca_iface_setup() -> Result<super::AtcaIfaceCfg, String> {
        let config_path = Path::new("config.toml");
        let config_string = read_to_string(config_path).expect("file not found");
        let config: Config = toml::from_str(&config_string).unwrap();
        match config.device.iface_type.as_str() {
            "i2c" => super::atca_iface_setup_i2c(
                config.device.device_type,
                config.device.wake_delay,
                config.device.rx_retries,
                Some(config.interface.slave_address),
                Some(config.interface.bus),
                Some(config.interface.baud),
            ),
            _ => Err("unsupported interface type".to_owned()),
        }
    }
    #[test]
    #[serial]
    fn atcab_init() {
        let atca_iface_cfg = atca_iface_setup();
        match atca_iface_cfg {
            Ok(x) => {
                assert_eq!(x.iface_type.to_string(), "AtcaI2cIface");
                assert_eq!(super::atcab_init(x).to_string(), "AtcaSuccess");
            }
            Err(e) => {
                panic!("Error reading config.toml file: {}", e);
            }
        };
        assert_eq!(super::atcab_release().to_string(), "AtcaSuccess");
    }
    #[test]
    #[serial]
    fn atcab_sha() {
        let atca_iface_cfg = atca_iface_setup();
        let mut digest: Vec<u8> = Vec::with_capacity(64);
        assert_eq!(atca_iface_cfg.is_ok(), true);
        assert_eq!(
            super::atcab_init(atca_iface_cfg.unwrap()).to_string(),
            "AtcaSuccess"
        );

        let test_message = "TestMessage";
        let message = test_message.as_bytes().to_vec();

        assert_eq!(
            super::atcab_sha(message, &mut digest).to_string(),
            "AtcaSuccess"
        );
        assert_eq!(super::atcab_release().to_string(), "AtcaSuccess");
    }
    #[test]
    #[serial]
    fn atcab_random() {
        let atca_iface_cfg = atca_iface_setup();
        let mut rand_out = Vec::with_capacity(32);
        assert_eq!(atca_iface_cfg.is_ok(), true);
        assert_eq!(
            super::atcab_init(atca_iface_cfg.unwrap()).to_string(),
            "AtcaSuccess"
        );
        assert_eq!(
            super::atcab_random(&mut rand_out).to_string(),
            "AtcaSuccess"
        );
        assert_eq!(super::atcab_release().to_string(), "AtcaSuccess");
    }
    #[test]
    #[serial]
    fn atcab_read_config_zone() {
        use crate::ATCA_ATECC_CONFIG_BUFFER_SIZE;
        let atca_iface_cfg = atca_iface_setup();
        let mut config_data = Vec::with_capacity(1024);
        assert_eq!(atca_iface_cfg.is_ok(), true);
        assert_eq!(
            super::atcab_init(atca_iface_cfg.unwrap()).to_string(),
            "AtcaSuccess"
        );
        assert_eq!(
            super::atcab_read_config_zone(&mut config_data).to_string(),
            "AtcaSuccess"
        );
        match super::atcab_get_device_type() {
            super::AtcaDeviceType::ATECC508A
            | super::AtcaDeviceType::ATECC608A
            | super::AtcaDeviceType::ATECC108A => {
                assert_eq!(config_data.len(), ATCA_ATECC_CONFIG_BUFFER_SIZE);
                assert_eq!(config_data[0], 0x01);
                assert_eq!(config_data[1], 0x23);
            }
            _ => (),
        };
        assert_eq!(super::atcab_release().to_string(), "AtcaSuccess");
    }
    #[test]
    #[serial]
    fn atcab_cmp_config_zone() {
        let atca_iface_cfg = atca_iface_setup();
        let mut config_data = Vec::with_capacity(1024);
        assert_eq!(atca_iface_cfg.is_ok(), true);
        assert_eq!(
            super::atcab_init(atca_iface_cfg.unwrap()).to_string(),
            "AtcaSuccess"
        );
        assert_eq!(
            super::atcab_read_config_zone(&mut config_data).to_string(),
            "AtcaSuccess"
        );
        let mut same_config = false;
        assert_eq!(
            super::atcab_cmp_config_zone(&mut config_data, &mut same_config).to_string(),
            "AtcaSuccess"
        );
        assert_eq!(same_config, true);
        assert_eq!(super::atcab_release().to_string(), "AtcaSuccess");
    }
    #[test]
    #[serial]
    fn atcab_configuration_is_locked() {
        let atca_iface_cfg = atca_iface_setup();
        assert_eq!(atca_iface_cfg.is_ok(), true);
        assert_eq!(
            super::atcab_init(atca_iface_cfg.unwrap()).to_string(),
            "AtcaSuccess"
        );
        let mut is_locked = false;
        assert_eq!(
            super::atcab_configuration_is_locked(&mut is_locked).to_string(),
            "AtcaSuccess"
        );
        assert_eq!(is_locked, true);
        assert_eq!(super::atcab_release().to_string(), "AtcaSuccess");
    }
}
