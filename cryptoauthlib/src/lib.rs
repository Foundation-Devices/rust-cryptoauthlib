#[macro_use]
extern crate strum_macros; // 0.10.0
#[macro_use]
extern crate lazy_static;

include!("types.rs");
include!("constants.rs");

mod hw_impl;
mod sw_impl;
mod atca_iface_cfg;

pub trait AteccDevice {
    /// Request ATECC to generate a vector of random bytes
    fn random(&self, rand_out: &mut Vec<u8>) -> AtcaStatus;
    /// Request ATECC to compute a message hash (SHA256)
    fn sha(&self, message: Vec<u8>, digest: &mut Vec<u8>) -> AtcaStatus;
    /// Execute a Nonce command in pass-through mode to load one of the
    /// device's internal buffers with a fixed value.
    /// For the ATECC608A, available targets are TempKey (32 or 64 bytes), Message
    /// Digest Buffer (32 or 64 bytes), or the Alternate Key Buffer (32 bytes). For
    /// all other devices, only TempKey (32 bytes) is available.
    fn nonce(&self, target: NonceTarget, data: &[u8]) -> AtcaStatus;
    /// Execute a Nonce command to generate a random nonce combining a host
    /// nonce and a device random number.
    fn nonce_rand(&self, host_nonce: &[u8], rand_out: &mut Vec<u8>) -> AtcaStatus;
    /// Request ATECC to generate a cryptographic key
    fn gen_key(&self, key_type: KeyType, slot_number: u8) -> AtcaStatus;
    /// Request ATECC to import a cryptographic key
    fn import_key(&self, key_type: KeyType, key_data: &[u8], slot_number: u8) -> AtcaStatus;
    /// Function to calculate the public key from an existing private key in a slot
    fn get_public_key(&self, slot_number: u8, public_key: &mut Vec<u8>) -> AtcaStatus;
    /// Request ATECC to generate an ECDSA signature
    fn sign_hash(&self, mode: SignMode, slot_number: u8, signature: &mut Vec<u8>) -> AtcaStatus;
    /// Request ATECC to verify ECDSA signature
    fn verify_hash(&self, mode: VerifyMode, hash: &[u8], signature: &[u8]) -> Result<bool, AtcaStatus>;
    /// Request ATECC to return own device type
    fn get_device_type(&self) -> AtcaDeviceType;
    /// Request ATECC to check if its configuration is locked.
    /// If true, a chip can be used for cryptographic operations
    fn configuration_is_locked(&self) -> Result<bool, AtcaStatus>;
    /// Request ATECC to check if its Data Zone is locked.
    /// If true, a chip can be used for cryptographic operations
    fn data_zone_is_locked(&self) -> Result<bool, AtcaStatus>;
    /// Request ATECC to read and return own configuration zone.
    /// Note: this function returns raw data, function get_config(..) implements a more
    /// structured return value.
    fn read_config_zone(&self, config_data: &mut Vec<u8>) -> AtcaStatus;
    /// Compare internal config zone contents vs. config_data.
    /// Diagnostic function.
    fn cmp_config_zone(&self, config_data: &mut Vec<u8>, same_config: &mut bool) -> AtcaStatus;
    /// Returns a structure containing configuration data read from ATECC
    /// during initialization of the AteccDevice object.
    fn get_config(&self, atca_slots: &mut Vec<AtcaSlot>) -> AtcaStatus;
    /// ATECC device instance destructor
    fn release(&self) -> AtcaStatus;
}

pub fn create_atecc_device(r_iface_cfg: AtcaIfaceCfg) -> Result<Box<dyn AteccDevice>, String> {
    match r_iface_cfg.devtype {
        AtcaDeviceType::AtcaTestDevSuccess 
        | AtcaDeviceType::AtcaTestDevFail => match sw_impl::AteccDevice::new(r_iface_cfg) {
            Ok(x) => Ok(Box::new(x)),
            Err(err) => Err(err),
        },
        AtcaDeviceType::AtcaDevUnknown => Err(String::from("Attempting to create an unknown device type")),
        _ => match hw_impl::AteccDevice::new(r_iface_cfg) {
            Ok(x) => Ok(Box::new(x)),
            Err(err) => Err(err),
        },
    }
}

// Setup an ATECC interface configuration (AtcaIfaceCfg)
// based on a device type and I2C parameters.
// This is a helper function, created for I2C exclusively.
// pub fn atca_iface_setup_i2c(
//     device_type: String,
//     wake_delay: u16,
//     rx_retries: i32,
//     // I2C salve address
//     slave_address: Option<u8>,
//     // I2C bus number
//     bus: Option<u8>,
//     // I2C baud rate
//     baud: Option<u32>,
// ) -> Result<AtcaIfaceCfg, String> {
//     let atca_iface_cfg = AtcaIfaceCfg {
//         iface_type: AtcaIfaceType::AtcaI2cIface,
//         devtype: match device_type.as_str() {
//             "atecc608a" => AtcaDeviceType::ATECC608A,
//             "atecc508a" => AtcaDeviceType::ATECC508A,
//             _ => {
//                 let e = "Unsupported device type ".to_owned() + device_type.as_str();
//                 return Err(e);
//             }
//         },
//         iface: AtcaIface {
//             atcai2c: AtcaIfaceI2c {
//                 // unwrap_or_else_return()?
//                 slave_address: match slave_address {
//                     Some(x) => x,
//                     _ => return Err("missing i2c slave address".to_owned()),
//                 },
//                 bus: match bus {
//                     Some(x) => x,
//                     _ => return Err("missing i2c bus".to_owned()),
//                 },
//                 baud: match baud {
//                     Some(x) => x,
//                     _ => return Err("missing i2c baud rate".to_owned()),
//                 },
//             },
//         },
//         rx_retries,
//         wake_delay,
//     };
//     Ok(atca_iface_cfg)
// }