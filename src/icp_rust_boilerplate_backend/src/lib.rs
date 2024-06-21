#[macro_use]
extern crate serde;
use candid::{Decode, Encode};
use ic_cdk::api::time;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{BoundedStorable, Cell, DefaultMemoryImpl, StableBTreeMap, Storable};
use std::{borrow::Cow, cell::RefCell};

type Memory = VirtualMemory<DefaultMemoryImpl>;
type IdCell = Cell<u64, Memory>;

#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct Mahasiswa {
    id: u64,
    nama: String,
    nim: String,
    jurusan: String,
    angkatan: u64,
    created_at: u64,
    updated_at: Option<u64>,
}

// Trait yang harus diimplementasikan untuk struct yang disimpan dalam struktur stabil
impl Storable for Mahasiswa {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

// Trait lain yang harus diimplementasikan untuk struct yang disimpan dalam struktur stabil
impl BoundedStorable for Mahasiswa {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );

    static ID_COUNTER: RefCell<IdCell> = RefCell::new(
        IdCell::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))), 0)
            .expect("Tidak dapat membuat counter")
    );

    static STORAGE: RefCell<StableBTreeMap<u64, Mahasiswa, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1)))
    ));
}

#[derive(candid::CandidType, Serialize, Deserialize, Default)]
struct MahasiswaPayload {
    nama: String,
    nim: String,
    jurusan: String,
    angkatan: u64,
}

#[ic_cdk::query]
fn get_mahasiswa(id: u64) -> Result<Mahasiswa, Error> {
    match _get_mahasiswa(&id) {
        Some(mahasiswa) => Ok(mahasiswa),
        None => Err(Error::NotFound {
            msg: format!("Mahasiswa dengan id={} tidak ditemukan", id),
        }),
    }
}

#[ic_cdk::update]
fn add_mahasiswa(payload: MahasiswaPayload) -> Option<Mahasiswa> {
    let id = ID_COUNTER
        .with(|counter| {
            let current_value = *counter.borrow().get();
            counter.borrow_mut().set(current_value + 1)
        })
        .expect("Tidak dapat mengincrement id counter");
    let mahasiswa = Mahasiswa {
        id,
        nama: payload.nama,
        nim: payload.nim,
        jurusan: payload.jurusan,
        angkatan: payload.angkatan,
        created_at: time(),
        updated_at: None,
    };
    do_insert(&mahasiswa);
    Some(mahasiswa)
}

#[ic_cdk::update]
fn update_mahasiswa(id: u64, payload: MahasiswaPayload) -> Result<Mahasiswa, Error> {
    match STORAGE.with(|service| service.borrow().get(&id)) {
        Some(mut mahasiswa) => {
            mahasiswa.nama = payload.nama;
            mahasiswa.nim = payload.nim;
            mahasiswa.jurusan = payload.jurusan;
            mahasiswa.angkatan = payload.angkatan;
            mahasiswa.updated_at = Some(time());
            do_insert(&mahasiswa);
            Ok(mahasiswa)
        }
        None => Err(Error::NotFound {
            msg: format!(
                "Tidak dapat mengupdate mahasiswa dengan id={}. Mahasiswa tidak ditemukan",
                id
            ),
        }),
    }
}

// Metode bantu untuk melakukan insert
fn do_insert(mahasiswa: &Mahasiswa) {
    STORAGE.with(|service| service.borrow_mut().insert(mahasiswa.id, mahasiswa.clone()));
}

#[ic_cdk::update]
fn delete_mahasiswa(id: u64) -> Result<Mahasiswa, Error> {
    match STORAGE.with(|service| service.borrow_mut().remove(&id)) {
        Some(mahasiswa) => Ok(mahasiswa),
        None => Err(Error::NotFound {
            msg: format!(
                "Tidak dapat menghapus mahasiswa dengan id={}. Mahasiswa tidak ditemukan.",
                id
            ),
        }),
    }
}

#[ic_cdk::query]
fn list_mahasiswa() -> Vec<Mahasiswa> {
    STORAGE.with(|service| {
        service.borrow().iter().map(|(_, mahasiswa)| mahasiswa).collect()
    })
}

#[ic_cdk::query]
fn find_mahasiswa_by_name(nama: String) -> Vec<Mahasiswa> {
    STORAGE.with(|service| {
        service
            .borrow()
            .iter()
            .filter(|(_, mahasiswa)| mahasiswa.nama.to_lowercase().contains(&nama.to_lowercase()))
            .map(|(_, mahasiswa)| mahasiswa)
            .collect()
    })
}

#[ic_cdk::query]
fn find_mahasiswa_by_nim(nim: String) -> Option<Mahasiswa> {
    STORAGE.with(|service| {
        service
            .borrow()
            .iter()
            .find(|(_, mahasiswa)| mahasiswa.nim == nim)
            .map(|(_, mahasiswa)| mahasiswa)
    })
}

#[derive(candid::CandidType, Deserialize, Serialize)]
enum Error {
    NotFound { msg: String },
}

// Metode bantu untuk mendapatkan mahasiswa berdasarkan id. Digunakan dalam get_mahasiswa/update_mahasiswa
fn _get_mahasiswa(id: &u64) -> Option<Mahasiswa> {
    STORAGE.with(|service| service.borrow().get(id))
}

// Dibutuhkan untuk menghasilkan candid
ic_cdk::export_candid!();
