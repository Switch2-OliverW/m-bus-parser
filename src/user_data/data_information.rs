use super::data_information::{self};
use super::variable_user_data::DataRecordError;

#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, PartialEq)]
pub struct DataInformationBlock<'a> {
    pub data_information_field: DataInformationField,
    pub data_information_field_extension: Option<DataInformationFieldExtensions<'a>>,
}

impl DataInformationBlock<'_> {
    #[must_use]
    pub fn get_size(&self) -> usize {
        let mut size = 1;
        if let Some(vife) = &self.data_information_field_extension {
            size += vife.len();
        }
        size
    }
}
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, PartialEq)]
pub struct DataInformationField {
    pub data: u8,
}

impl From<data_information::DataInformationError> for DataRecordError {
    fn from(error: data_information::DataInformationError) -> Self {
        DataRecordError::DataInformationError(error)
    }
}

impl From<u8> for DataInformationField {
    fn from(data: u8) -> Self {
        DataInformationField { data }
    }
}

impl From<u8> for DataInformationFieldExtension {
    fn from(data: u8) -> Self {
        DataInformationFieldExtension { data }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, PartialEq)]
#[repr(transparent)]
pub struct DataInformationFieldExtension {
    pub data: u8,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct DataInformationFieldExtensions<'a>(&'a [u8]);
impl<'a> DataInformationFieldExtensions<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self(data)
    }
}

impl<'a> Iterator for DataInformationFieldExtensions<'a> {
    type Item = DataInformationFieldExtension;
    fn next(&mut self) -> Option<Self::Item> {
        let (head, tail) = self.0.split_first()?;
        self.0 = tail;
        Some(DataInformationFieldExtension { data: *head })
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.0.len(), Some(self.0.len()))
    }
}
impl<'a> ExactSizeIterator for DataInformationFieldExtensions<'a> {}
impl<'a> DoubleEndedIterator for DataInformationFieldExtensions<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let (end, start) = self.0.split_last()?;
        self.0 = start;
        Some(DataInformationFieldExtension { data: *end })
    }
}

impl<'a> TryFrom<&'a [u8]> for DataInformationBlock<'a> {
    type Error = DataInformationError;

    fn try_from(data: &'a [u8]) -> Result<Self, DataInformationError> {
        let Some((dif_byte, data)) = data.split_first() else {
            return Err(DataInformationError::NoData);
        };
        let dif = DataInformationField::from(*dif_byte);

        let length = data.iter().take_while(|&&u8| u8 & 0x80 != 0).count();
        let offset = length + 1;
        match () {
            () if dif.has_extension() && offset > MAXIMUM_DATA_INFORMATION_SIZE => {
                Err(DataInformationError::DataTooLong)
            }
            () if dif.has_extension() && offset > data.len() => {
                Err(DataInformationError::DataTooShort)
            }
            () if dif.has_extension() => Ok(DataInformationBlock {
                data_information_field: dif,
                data_information_field_extension: Some(DataInformationFieldExtensions::new(
                    &data[..offset],
                )),
            }),
            () => Ok(DataInformationBlock {
                data_information_field: dif,
                data_information_field_extension: None,
            }),
        }
    }
}

impl DataInformationField {
    fn has_extension(&self) -> bool {
        self.data & 0x80 != 0
    }
}

impl DataInformationFieldExtension {
    fn special_function(&self) -> SpecialFunctions {
        match self.data {
            0x0F => SpecialFunctions::ManufacturerSpecific,
            0x1F => SpecialFunctions::MoreRecordsFollow,
            0x2F => SpecialFunctions::IdleFiller,
            0x7F => SpecialFunctions::GlobalReadoutRequest,
            _ => SpecialFunctions::Reserved,
        }
    }
}
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct DataInformation {
    pub storage_number: u64,
    pub function_field: FunctionField,
    pub data_field_coding: DataFieldCoding,
    pub data_information_extension: Option<DataInformationExtensionField>,
    pub size: usize,
}

#[cfg(feature = "std")]
impl std::fmt::Display for DataInformation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{},{},{}",
            self.storage_number, self.function_field, self.data_field_coding
        )
    }
}

const MAXIMUM_DATA_INFORMATION_SIZE: usize = 11;
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct DataInformationExtensionField {}

#[derive(Debug, PartialEq)]
pub enum DataInformationError {
    NoData,
    DataTooLong,
    DataTooShort,
    InvalidValueInformation,
}

impl TryFrom<&DataInformationBlock<'_>> for DataInformation {
    type Error = DataInformationError;

    fn try_from(
        data_information_block: &DataInformationBlock,
    ) -> Result<Self, DataInformationError> {
        let dif = data_information_block.data_information_field.data;
        let possible_difes = &data_information_block.data_information_field_extension;
        let mut storage_number = u64::from((dif & 0b0100_0000) >> 6);

        let mut extension_bit = dif & 0x80 != 0;
        let mut extension_index = 1;
        let mut _tariff = 0;
        let mut _sub_unit = 0;
        let mut first_dife = None;

        if let Some(difes) = possible_difes {
            first_dife = difes.clone().next();
            for dife in difes.clone() {
                if extension_index > MAXIMUM_DATA_INFORMATION_SIZE {
                    return Err(DataInformationError::DataTooLong);
                }
                let dife = dife.data;
                storage_number += u64::from(dife & 0x0f) << ((extension_index * 4) + 1);
                _sub_unit += u32::from((dife & 0x40) >> 6) << extension_index;
                _tariff += u64::from((dife & 0x30) >> 4) << (extension_index * 2);
                extension_bit = dife & 0x80 != 0;
                extension_index += 1;
            }
        }

        let function_field = match (dif & 0b0011_0000) >> 4 {
            0b00 => FunctionField::InstantaneousValue,
            0b01 => FunctionField::MaximumValue,
            0b10 => FunctionField::MinimumValue,
            _ => FunctionField::ValueDuringErrorState,
        };
        let data_field_coding = match dif & 0b0000_1111 {
            0b0000 => DataFieldCoding::NoData,
            0b0001 => DataFieldCoding::Integer8Bit,
            0b0010 => DataFieldCoding::Integer16Bit,
            0b0011 => DataFieldCoding::Integer24Bit,
            0b0100 => DataFieldCoding::Integer32Bit,
            0b0101 => DataFieldCoding::Real32Bit,
            0b0110 => DataFieldCoding::Integer48Bit,
            0b0111 => DataFieldCoding::Integer64Bit,
            0b1000 => DataFieldCoding::SelectionForReadout,
            0b1001 => DataFieldCoding::BCD2Digit,
            0b1010 => DataFieldCoding::BCD4Digit,
            0b1011 => DataFieldCoding::BCD6Digit,
            0b1100 => DataFieldCoding::BCD8Digit,
            0b1101 => DataFieldCoding::VariableLength,
            0b1110 => DataFieldCoding::BCDDigit12,
            0b1111 => DataFieldCoding::SpecialFunctions(first_dife.unwrap().special_function()),
            _ => unreachable!(), // This case should never occur due to the 4-bit width
        };

        Ok(DataInformation {
            storage_number,
            function_field,
            data_field_coding,
            data_information_extension: if extension_bit {
                Some(DataInformationExtensionField {})
            } else {
                None
            },
            size: extension_index,
        })
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(into = "String"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextUnit<'a>(&'a [u8]);
impl<'a> TextUnit<'a> {
    pub fn new(input: &'a [u8]) -> Self {
        Self(input)
    }
}

impl PartialEq<str> for TextUnit<'_> {
    fn eq(&self, other: &str) -> bool {
        self.0.iter().eq(other.as_bytes().iter().rev())
    }
}
#[cfg(any(feature = "serde", feature = "std"))]
impl From<TextUnit<'_>> for String {
    fn from(value: TextUnit<'_>) -> Self {
        let value: Vec<u8> = value.0.iter().copied().rev().collect();
        String::from_utf8(value).unwrap()
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, PartialEq)]
pub enum Month {
    January,
    February,
    March,
    April,
    May,
    June,
    July,
    August,
    September,
    October,
    November,
    December,
}

#[cfg(feature = "std")]
impl std::fmt::Display for Month {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Month::January => write!(f, "Jan"),
            Month::February => write!(f, "Feb"),
            Month::March => write!(f, "Mar"),
            Month::April => write!(f, "Apr"),
            Month::May => write!(f, "May"),
            Month::June => write!(f, "Jun"),
            Month::July => write!(f, "Jul"),
            Month::August => write!(f, "Aug"),
            Month::September => write!(f, "Sep"),
            Month::October => write!(f, "Oct"),
            Month::November => write!(f, "Nov"),
            Month::December => write!(f, "Dec"),
        }
    }
}

pub type Year = u16;
pub type DayOfMonth = u8;
pub type Hour = u8;
pub type Minute = u8;
pub type Second = u8;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, PartialEq)]
pub enum SingleOrEvery<T> {
    Single(T),
    Every(),
}

#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, PartialEq)]
pub enum DataType<'a> {
    Text(TextUnit<'a>),
    Number(f64),
        Date(
        SingleOrEvery<DayOfMonth>,
        SingleOrEvery<Month>,
        SingleOrEvery<Year>,
    ),
    DateTime(
        SingleOrEvery<DayOfMonth>,
        SingleOrEvery<Month>,
        SingleOrEvery<Year>,
        SingleOrEvery<Hour>,
        SingleOrEvery<Minute>,
    ),
    DateTimeWithSeconds(
        SingleOrEvery<DayOfMonth>,
        SingleOrEvery<Month>,
        SingleOrEvery<Year>,
        SingleOrEvery<Hour>,
        SingleOrEvery<Minute>,
        SingleOrEvery<Second>,
    ),
}
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(PartialEq, Debug)]
pub struct Data<'a> {
    value: Option<DataType<'a>>,
    size: usize,
}

#[cfg(feature = "std")]
impl<T: std::fmt::Display> std::fmt::Display for SingleOrEvery<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SingleOrEvery::Single(value) => write!(f, "{}", value),
            SingleOrEvery::Every() => write!(f, "Every"),
        }
    }
}

#[cfg(feature = "std")]
impl std::fmt::Display for Data<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.value {
            Some(value) => match value {
                &DataType::Text(text) => {
                    let text: String = text.into();
                    write!(f, "{}", text)
                }
                DataType::Number(value) => write!(f, "{}", value),
                DataType::Date(day, month, year) => write!(f, "{}/{}/{}", day, month, year),
                DataType::DateTime(day, month, year, hour, minute) => {
                    write!(f, "{}/{}/{} {}:{}:00", day, month, year, hour, minute)
                }
                DataType::DateTimeWithSeconds(day, month, year, hour, minute, second) => {
                    write!(
                        f,
                        "{}/{}/{} {}:{}:{}",
                        day, month, year, hour, minute, second
                    )
                }
            },
            None => write!(f, "No Data"),
        }
    }
}

impl Data<'_> {
    #[must_use]
    pub fn get_size(&self) -> usize {
        self.size
    }
}

impl DataFieldCoding {
    pub fn parse<'a>(&self, input: &'a [u8]) -> Result<Data<'a>, DataRecordError> {
        match self {
            DataFieldCoding::NoData => Ok(Data {
                value: None,
                size: 0,
            }),

            DataFieldCoding::Integer8Bit => {
                if input.is_empty() {
                    return Err(DataRecordError::InsufficientData);
                }
                let value = input[0] as i8;
                Ok(Data {
                    value: Some(DataType::Number(f64::from(value))),
                    size: 1,
                })
            }

            DataFieldCoding::Integer16Bit => {
                if input.len() < 2 {
                    return Err(DataRecordError::InsufficientData);
                }
                let value = i16::from_le_bytes(input[0..2].try_into().unwrap());

                Ok(Data {
                    value: Some(DataType::Number(f64::from(value))),
                    size: 2,
                })
            }

            DataFieldCoding::Integer24Bit => {
                if input.len() < 3 {
                    return Err(DataRecordError::InsufficientData);
                }
                let value =
                    i32::from(input[0]) | (i32::from(input[1]) << 8) | (i32::from(input[2]) << 16);
                Ok(Data {
                    value: Some(DataType::Number(f64::from(value))),
                    size: 3,
                })
            }

            DataFieldCoding::Integer32Bit => {
                if input.len() < 4 {
                    return Err(DataRecordError::InsufficientData);
                }
                let value = i32::from_le_bytes(input[0..4].try_into().unwrap());
                Ok(Data {
                    value: Some(DataType::Number(f64::from(value))),
                    size: 4,
                })
            }

            DataFieldCoding::Real32Bit => {
                if input.len() < 4 {
                    return Err(DataRecordError::InsufficientData);
                }
                let value = f32::from_le_bytes(input[0..4].try_into().unwrap());
                Ok(Data {
                    value: Some(DataType::Number(f64::from(value))),
                    size: 4,
                })
            }

            DataFieldCoding::Integer48Bit => {
                if input.len() < 6 {
                    return Err(DataRecordError::InsufficientData);
                }
                let value = i64::from(input[0])
                    | (i64::from(input[1]) << 8)
                    | (i64::from(input[2]) << 16)
                    | (i64::from(input[3]) << 24)
                    | (i64::from(input[4]) << 32)
                    | (i64::from(input[5]) << 40);
                Ok(Data {
                    value: Some(DataType::Number(value as f64)),
                    size: 6,
                })
            }

            DataFieldCoding::Integer64Bit => {
                if input.len() < 8 {
                    return Err(DataRecordError::InsufficientData);
                }
                let value = i64::from_le_bytes(input[0..8].try_into().unwrap());
                Ok(Data {
                    value: Some(DataType::Number(value as f64)),
                    size: 8,
                })
            }

            DataFieldCoding::SelectionForReadout => Ok(Data {
                value: None,
                size: 0,
            }),

            DataFieldCoding::BCD2Digit => {
                if input.is_empty() {
                    return Err(DataRecordError::InsufficientData);
                }
                let value = bcd_to_u8(input[0]);
                Ok(Data {
                    value: Some(DataType::Number(f64::from(value))),
                    size: 1,
                })
            }

            DataFieldCoding::BCD4Digit => {
                if input.len() < 2 {
                    return Err(DataRecordError::InsufficientData);
                }
                let value = bcd_to_u16(&input[0..2]);
                Ok(Data {
                    value: Some(DataType::Number(f64::from(value))),
                    size: 2,
                })
            }

            DataFieldCoding::BCD6Digit => {
                if input.len() < 3 {
                    return Err(DataRecordError::InsufficientData);
                }
                let value = bcd_to_u32(&input[0..3]);
                Ok(Data {
                    value: Some(DataType::Number(f64::from(value))),
                    size: 3,
                })
            }

            DataFieldCoding::BCD8Digit => {
                if input.len() < 4 {
                    return Err(DataRecordError::InsufficientData);
                }
                let value = bcd_to_u32(&input[0..4]);
                Ok(Data {
                    value: Some(DataType::Number(f64::from(value))),
                    size: 4,
                })
            }

            DataFieldCoding::VariableLength => {
                let mut length = input[0];
                match input[0] {
                    0x00..=0xBF => Ok(Data {
                        value: Some(DataType::Text(TextUnit::new(&input[1..(length as usize)]))),
                        size: length as usize + 1,
                    }),
                    0xC0..=0xD9 => {
                        length -= 0xC0;
                        let is_negative = input[0] > 0xC9;
                        let sign = if is_negative { -1.0 } else { 1.0 };
                        if length as usize > input.len() {
                            return Err(DataRecordError::InsufficientData);
                        }
                        match length {
                            2 => {
                                let value = bcd_to_u8(input[1]);
                                Ok(Data {
                                    value: Some(DataType::Number(sign * f64::from(value))),
                                    size: 2,
                                })
                            }
                            4 => {
                                let value = bcd_to_u16(&input[1..3]);
                                Ok(Data {
                                    value: Some(DataType::Number(sign * f64::from(value))),
                                    size: 4,
                                })
                            }
                            6 => {
                                let value = bcd_to_u32(&input[1..5]);
                                Ok(Data {
                                    value: Some(DataType::Number(sign * f64::from(value))),
                                    size: 6,
                                })
                            }
                            8 => {
                                let value = bcd_to_u32(&input[1..5]);
                                Ok(Data {
                                    value: Some(DataType::Number(sign * f64::from(value))),
                                    size: 8,
                                })
                            }
                            _ => {
                                todo!("8-bit text string according to ISO/IEC 8859-1 of length greater than {}", length);
                            }
                        }
                    }
                    0xE0..=0xE9 => {
                        length -= 0xE0;
                        todo!("0xE0-0xE9 not implemented for length {}", length);
                    }
                    0xF0..=0xF4 => {
                        length -= 0xF0;
                        todo!("0xF0-0xF4 not implemented for length {}", length);
                    }
                    0xF5 => {
                        length = 6;
                        todo!("0xF5 not implemented for length {}", length);
                    }
                    0xF6 => {
                        length = 8;
                        todo!("0xF6 not implemented for length {}", length);
                    }
                    _ => {
                        todo!(
                            "Variable length parsing for length: {} is a resreved value",
                            length
                        );
                    }
                }
            }

            DataFieldCoding::BCDDigit12 => {
                if input.len() < 6 {
                    return Err(DataRecordError::InsufficientData);
                }
                let value = bcd_to_u48(&input[0..6]);
                Ok(Data {
                    value: Some(DataType::Number(value as f64)),
                    size: 6,
                })
            }

            DataFieldCoding::SpecialFunctions(_code) => {
                // Special functions parsing based on the code
                todo!()
            }

            DataFieldCoding::DateTypeG => {
                if input.len() < 2 {
                    return Err(DataRecordError::InsufficientData);
                }
                // lowest five bits of first byte is day, 0 means every day
                let day = if input[0] & 0x1F == 0 {
                    SingleOrEvery::Every()
                } else {
                    SingleOrEvery::Single(input[0] & 0x1F)
                };

                // the first four bits of the second byte is month, 15 means every month
                let month = if input[1] & 0xF0 == 0xF0 {
                    SingleOrEvery::Every()
                } else {
                    SingleOrEvery::Single(match input[1] & 0xF {
                        0x0 => Month::January,
                        0x1 => Month::February,
                        0x2 => Month::March,
                        0x3 => Month::April,
                        0x4 => Month::May,
                        0x5 => Month::June,
                        0x6 => Month::July,
                        0x7 => Month::August,
                        0x8 => Month::September,
                        0x9 => Month::October,
                        0xA => Month::November,
                        0xB => Month::December,
                        _ => return Err(DataRecordError::InvalidData),
                    })
                };
                let year = u16::from(input[1] & 0xF0) | (u16::from(input[0] & 0xE0) << 1);
                let year = if year == 0x7F {
                    SingleOrEvery::Every()
                } else {
                    SingleOrEvery::Single(year)
                };

                Ok(Data {
                    value: Some(DataType::Date(day, month, year)),
                    size: 2,
                })
            }
            DataFieldCoding::DateTimeTypeF => {
                if input.len() < 4 {
                    return Err(DataRecordError::InsufficientData);
                }
                let minutes = if input[0] == 0x3F {
                    SingleOrEvery::Every()
                } else {
                    SingleOrEvery::Single(input[0] & 0x3F)
                };

                let hour = if input[1] == 0x1F {
                    SingleOrEvery::Every()
                } else {
                    SingleOrEvery::Single(input[1] & 0x1F)
                };

                let day = if input[2] == 0x1F {
                    SingleOrEvery::Every()
                } else {
                    SingleOrEvery::Single(input[2] & 0x1F)
                };

                let month = if input[3] == 0x0F {
                    SingleOrEvery::Every()
                } else {
                    SingleOrEvery::Single(match input[3] & 0x0F {
                        0x0 => Month::January,
                        0x1 => Month::February,
                        0x2 => Month::March,
                        0x3 => Month::April,
                        0x4 => Month::May,
                        0x5 => Month::June,
                        0x6 => Month::July,
                        0x7 => Month::August,
                        0x8 => Month::September,
                        0x9 => Month::October,
                        0xA => Month::November,
                        0xB => Month::December,
                        _ => return Err(DataRecordError::InvalidData),
                    })
                };

                // year [21 to 23; 28 to 31]
                let year = u16::from(input[3] & 0xF0) | (u16::from(input[2] & 0xE0) << 1);
                let year = if year == 0x7F {
                    SingleOrEvery::Every()
                } else {
                    SingleOrEvery::Single(year)
                };
                Ok(Data {
                    value: Some(DataType::DateTime(day, month, year, hour, minutes)),
                    size: 4,
                })
            }
            DataFieldCoding::DateTimeTypeJ => {
                todo!();
                Ok(Data {
                    value: Some(DataType::Date(
                        SingleOrEvery::Every(),
                        SingleOrEvery::Every(),
                        SingleOrEvery::Every(),
                    )),
                    size: 4,
                })
            }
            DataFieldCoding::DateTimeTypeI => {
                todo!();
                Ok(Data {
                    value: Some(DataType::Date(
                        SingleOrEvery::Every(),
                        SingleOrEvery::Every(),
                        SingleOrEvery::Every(),
                    )),
                    size: 4,
                })
            }
        }
    }
}

fn bcd_to_u8(bcd: u8) -> u8 {
    (bcd >> 4) * 10 + (bcd & 0x0F)
}

fn bcd_to_u16(bcd: &[u8]) -> u16 {
    match bcd.len() {
        1 => u16::from(bcd_to_u8(bcd[0])),
        2 => (u16::from(bcd_to_u8(bcd[1])) * 100 + u16::from(bcd_to_u8(bcd[0]))) as u16,
        _ => panic!(
            "BCD input length must be either 1 or 2 but got {}",
            bcd.len()
        ),
    }
}

fn bcd_to_u32(bcd: &[u8]) -> u32 {
    match bcd.len() {
        3 => (u32::from(bcd_to_u8(bcd[2])) * 10000 + u32::from(bcd_to_u16(&bcd[0..2]))) as u32,
        4 => (u32::from(bcd_to_u16(&bcd[2..4])) * 10000 + u32::from(bcd_to_u16(&bcd[0..2]))) as u32,
        _ => panic!(
            "BCD input length must be either 3 or 4 but got {}",
            bcd.len()
        ),
    }
}

fn bcd_to_u48(bcd: &[u8]) -> u64 {
    (u64::from(bcd_to_u32(&bcd[2..6])) * 1_000_000 + u64::from(bcd_to_u16(&bcd[0..2]))) as u64
}

impl DataInformation {
    #[must_use]
    pub fn get_size(&self) -> usize {
        self.size
    }
}
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FunctionField {
    InstantaneousValue,
    MaximumValue,
    MinimumValue,
    ValueDuringErrorState,
}

#[cfg(feature = "std")]
impl std::fmt::Display for FunctionField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FunctionField::InstantaneousValue => write!(f, "Inst"),
            FunctionField::MaximumValue => write!(f, "Max"),
            FunctionField::MinimumValue => write!(f, "Min"),
            FunctionField::ValueDuringErrorState => write!(f, "Value During Error State"),
        }
    }
}
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SpecialFunctions {
    ManufacturerSpecific,
    MoreRecordsFollow,
    IdleFiller,
    Reserved,
    GlobalReadoutRequest,
}

pub struct Value {
    pub data: f64,
    pub byte_size: usize,
}

impl DataFieldCoding {
    #[must_use]
    pub fn extract_from_bytes(&self, data: &[u8]) -> Value {
        match *self {
            DataFieldCoding::Real32Bit => Value {
                data: f64::from(f32::from_le_bytes([data[0], data[1], data[2], data[3]])),
                byte_size: 4,
            },
            DataFieldCoding::Integer8Bit => Value {
                data: f64::from(data[0]),
                byte_size: 1,
            },
            DataFieldCoding::Integer16Bit => Value {
                data: f64::from(u16::from(data[1]) << 8 | u16::from(data[0])),
                byte_size: 2,
            },
            DataFieldCoding::Integer24Bit => Value {
                data: f64::from(
                    u32::from(data[2]) << 16 | u32::from(data[1]) << 8 | u32::from(data[0]),
                ),
                byte_size: 3,
            },
            DataFieldCoding::Integer32Bit => Value {
                data: f64::from(
                    u32::from(data[3]) << 24
                        | u32::from(data[2]) << 16
                        | u32::from(data[1]) << 8
                        | u32::from(data[0]),
                ),
                byte_size: 4,
            },
            DataFieldCoding::Integer48Bit => Value {
                data: (u64::from(data[5]) << 40
                    | u64::from(data[4]) << 32
                    | u64::from(data[3]) << 24
                    | u64::from(data[2]) << 16
                    | u64::from(data[1]) << 8
                    | u64::from(data[0])) as f64,
                byte_size: 6,
            },
            DataFieldCoding::Integer64Bit => Value {
                data: (u64::from(data[7]) << 56
                    | u64::from(data[6]) << 48
                    | u64::from(data[5]) << 40
                    | u64::from(data[4]) << 32
                    | u64::from(data[3]) << 24
                    | u64::from(data[2]) << 16
                    | u64::from(data[1]) << 8
                    | u64::from(data[0])) as f64,
                byte_size: 8,
            },
            DataFieldCoding::BCD2Digit => Value {
                data: (f64::from(data[0] >> 4) * 10.0) + f64::from(data[0] & 0x0F),
                byte_size: 1,
            },
            DataFieldCoding::BCD4Digit => Value {
                data: (f64::from(data[1] >> 4) * 1000.0)
                    + (f64::from(data[1] & 0x0F) * 100.0)
                    + (f64::from(data[0] >> 4) * 10.0)
                    + f64::from(data[0] & 0x0F),
                byte_size: 2,
            },
            DataFieldCoding::BCD6Digit => Value {
                data: (f64::from(data[2] >> 4) * 100_000.0)
                    + (f64::from(data[2] & 0x0F) * 10000.0)
                    + (f64::from(data[1] >> 4) * 1000.0)
                    + (f64::from(data[1] & 0x0F) * 100.0)
                    + (f64::from(data[0] >> 4) * 10.0)
                    + f64::from(data[0] & 0x0F),
                byte_size: 3,
            },
            DataFieldCoding::BCD8Digit => Value {
                data: (f64::from(data[3] >> 4) * 10_000_000.0)
                    + (f64::from(data[3] & 0x0F) * 1_000_000.0)
                    + (f64::from(data[2] >> 4) * 100_000.0)
                    + (f64::from(data[2] & 0x0F) * 10000.0)
                    + (f64::from(data[1] >> 4) * 1000.0)
                    + (f64::from(data[1] & 0x0F) * 100.0)
                    + (f64::from(data[0] >> 4) * 10.0)
                    + f64::from(data[0] & 0x0F),
                byte_size: 4,
            },
            DataFieldCoding::BCDDigit12 => Value {
                data: (f64::from(data[5] >> 4) * 100_000_000_000.0)
                    + (f64::from(data[5] & 0x0F) * 10_000_000_000.0)
                    + (f64::from(data[4] >> 4) * 1_000_000_000.0)
                    + (f64::from(data[4] & 0x0F) * 100_000_000.0)
                    + (f64::from(data[3] >> 4) * 10_000_000.0)
                    + (f64::from(data[3] & 0x0F) * 1_000_000.0)
                    + (f64::from(data[2] >> 4) * 100_000.0)
                    + (f64::from(data[2] & 0x0F) * 10000.0)
                    + (f64::from(data[1] >> 4) * 1000.0)
                    + (f64::from(data[1] & 0x0F) * 100.0)
                    + (f64::from(data[0] >> 4) * 10.0)
                    + f64::from(data[0] & 0x0F),
                byte_size: 6,
            },
            DataFieldCoding::NoData => Value {
                data: 0.0,
                byte_size: 0,
            },
            DataFieldCoding::SelectionForReadout => Value {
                data: 0.0,
                byte_size: 0,
            },
            DataFieldCoding::SpecialFunctions(_) => Value {
                data: 0.0,
                byte_size: 0,
            },
            DataFieldCoding::VariableLength => Value {
                data: 0.0,
                byte_size: 0,
            },
            DataFieldCoding::DateTypeG => Value {
                data: 0.0,
                byte_size: 0,
            },
            DataFieldCoding::DateTimeTypeF => Value {
                data: 0.0,
                byte_size: 0,
            },
            DataFieldCoding::DateTimeTypeJ => Value {
                data: 0.0,
                byte_size: 0,
            },
            DataFieldCoding::DateTimeTypeI => Value {
                data: 0.0,
                byte_size: 0,
            },
        }
    }
}
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DataFieldCoding {
    NoData,
    Integer8Bit,
    Integer16Bit,
    Integer24Bit,
    Integer32Bit,
    Real32Bit,
    Integer48Bit,
    Integer64Bit,
    SelectionForReadout,
    BCD2Digit,
    BCD4Digit,
    BCD6Digit,
    BCD8Digit,
    VariableLength,
    BCDDigit12,
    SpecialFunctions(SpecialFunctions),
    DateTypeG,
    DateTimeTypeF,
    DateTimeTypeJ,
    DateTimeTypeI,
}

#[cfg(feature = "std")]
impl std::fmt::Display for DataFieldCoding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataFieldCoding::NoData => write!(f, "No Data"),
            DataFieldCoding::Integer8Bit => write!(f, "8-bit Integer"),
            DataFieldCoding::Integer16Bit => write!(f, "16-bit Integer"),
            DataFieldCoding::Integer24Bit => write!(f, "24-bit Integer"),
            DataFieldCoding::Integer32Bit => write!(f, "32-bit Integer"),
            DataFieldCoding::Real32Bit => write!(f, "32-bit Real"),
            DataFieldCoding::Integer48Bit => write!(f, "48-bit Integer"),
            DataFieldCoding::Integer64Bit => write!(f, "64-bit Integer"),
            DataFieldCoding::SelectionForReadout => write!(f, "Selection for Readout"),
            DataFieldCoding::BCD2Digit => write!(f, "BCD 2-digit"),
            DataFieldCoding::BCD4Digit => write!(f, "BCD 4-digit"),
            DataFieldCoding::BCD6Digit => write!(f, "BCD 6-digit"),
            DataFieldCoding::BCD8Digit => write!(f, "BCD 8-digit"),
            DataFieldCoding::VariableLength => write!(f, "Variable Length"),
            DataFieldCoding::BCDDigit12 => write!(f, "BCD 12-digit"),
            DataFieldCoding::DateTypeG => write!(f, "Date Type G"),
            DataFieldCoding::DateTimeTypeF => write!(f, "Date Time Type F"),
            DataFieldCoding::DateTimeTypeJ => write!(f, "Date Time Type J"),
            DataFieldCoding::DateTimeTypeI => write!(f, "Date Time Type I"),
            DataFieldCoding::SpecialFunctions(code) => write!(f, "Special Functions ({:?})", code),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    #[test]
    fn test_data_information() {
        let data = [0x13_u8];
        let result = DataInformationBlock::try_from(data.as_slice());
        let result = DataInformation::try_from(&result.unwrap());
        assert_eq!(
            result,
            Ok(DataInformation {
                storage_number: 0,
                function_field: FunctionField::MaximumValue,
                data_field_coding: DataFieldCoding::Integer24Bit,
                data_information_extension: None,
                size: 1,
            })
        );
    }

    #[test]
    fn reverse_text_unit() {
        let original_value = [0x6c, 0x61, 0x67, 0x69];
        let parsed = TextUnit::new(&original_value);
        assert_eq!(&parsed, "igal");
    }

    #[test]
    fn test_invalid_data_information() {
        let data = [
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        ];
        let result = DataInformationBlock::try_from(data.as_slice());
        assert_eq!(result, Err(DataInformationError::DataTooLong));
    }

    #[test]
    fn test_longest_data_information_not_too_long() {
        let data = [
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01,
        ];
        let result = DataInformationBlock::try_from(data.as_slice());
        assert_ne!(result, Err(DataInformationError::DataTooLong));
    }

    #[test]
    fn test_short_data_information() {
        let data = [0xFF];
        let result = DataInformationBlock::try_from(data.as_slice());
        assert_eq!(result, Err(DataInformationError::DataTooShort));
    }

    #[test]
    fn test_data_inforamtion1() {
        let data = [178, 1];
        let result = DataInformationBlock::try_from(data.as_slice());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().get_size(), 2);
    }
}
