# Kebijakan Keamanan (Security Policy)

Keamanan pengguna adalah prioritas utama dalam desain **Gaggle**. Karena Gaggle adalah agent AI *self-hosted* yang bekerja langsung pada sistem file nyata, terminal (PTY), browser sandbox, dan ekstensi MCP di mesin pengguna, setiap celah keamanan pada lapisan izin (*permission engine*), eksekusi kode, maupun penyimpanan kredensial kami tangani secara serius.

---

## Versi yang Didukung (Supported Versions)

Saat ini Gaggle berada dalam tahap pengembangan aktif (`0.1.x`). Pembaruan keamanan diberikan pada rilis terbaru dan cabang utama (`main`).

| Versi | Didukung Pembaruan Keamanan |
| :--- | :--- |
| `0.1.x` (terbaru) | :white_check_mark: Ya |
| `< 0.1.0` | :x: Tidak |

---

## Prinsip Keamanan Arsitektur Gaggle

Dalam mengembangkan maupun mengaudit Gaggle, arsitektur berikut merupakan batas keamanan (*security boundary*) utama yang wajib dijaga:

1. **Kredensial Tidak Pernah Disimpan di SQLite**
   - API key, token OAuth provider LLM, dan rahasia environment variable ekstensi (`env_secret_refs`) **dilarang** disimpan dalam bentuk *plaintext* di database `gaggle.db`.
   - Database hanya menyimpan pengenal referensi (`secret_ref`) yang menunjuk ke OS Keychain (macOS Keychain, Linux Secret Service / KWallet, Windows Credential Manager).

2. **7-Step Permission Engine & Command Rules**
   - Mode izin default untuk setiap percakapan baru adalah `ask` (bukan *autonomous* / `full`).
   - Aturan perintah shell (`command_rules`) dengan aksi `deny` bersifat **mutlak** dan wajib memblokir eksekusi bahkan ketika percakapan berada dalam mode `full`.
   - Mode `chat_only` wajib menolak seluruh pemanggilan tool tanpa terkecuali.

3. **Isolasi Mode Kerja & Git Worktree**
   - Pada mode `readonly`, agent tidak boleh memodifikasi file di dalam `root_path` proyek.
   - Pada mode `worktree`, seluruh operasi tulis dan eksekusi tool harus terikat pada direktori Git Worktree yang terisolasi untuk percakapan tersebut.

4. **Code Mode & Extension Sandboxing**
   - Eksekusi skrip di dalam Code Mode (`code_executions`) dibatasi oleh batas memori (`memory_limit_mb`) dan batas waktu (`timeout_ms`).
   - Ekstensi (`extensions`) hanya boleh mengakses kemampuan (`fs.read`, `fs.write`, `shell`, `net`, `browser`, `memory`) yang telah diberikan izin eksplisit di tabel `extension_permissions`.

5. **Otentikasi Perangkat (`devices`)**
   - Token akses untuk perangkat jarak jauh (misalnya akses browser dari ponsel ke server lokal) hanya disimpan sebagai *cryptographic hash* (`token_hash`) dan dapat dicabut kapan saja melalui kolom `revoked_at`.

---

## Melaporkan Celah Keamanan (Reporting a Vulnerability)

**Mohon JANGAN melaporkan celah keamanan melalui GitHub Issue publik.**

Jika Anda menemukan potensi kerentanan keamanan pada Gaggle (seperti *permission bypass*, *command injection*, *path traversal* di luar workspace proyek, atau kebocoran kredensial):

1. **Saluran Pelaporan Privat**:
   - Gunakan fitur **[GitHub Private Vulnerability Reporting / Security Advisories](https://github.com/frengky/gaggle/security/advisories/new)** pada repositori ini, atau hubungi pengelola utama proyek secara langsung.

2. **Informasi yang Perlu Disertakan**:
   - Deskripsi rinci mengenai celah keamanan dan komponen crate yang terdampak (`gaggle-core`, `gaggle-db`, `gaggle-tools`, `gaggle-mcp`, `gaggle-server`, dll.).
   - Langkah-langkah reproduksi (*Proof of Concept* / PoC) yang jelas.
   - Dampak potensial jika celah tersebut dieksploitasi (mis. eksekusi perintah tanpa *approval* pada mode `ask`).
   - Usulan perbaikan atau mitigasi (jika ada).

3. **Waktu Respons & Proses Penanganan**:
   - **Konfirmasi Penerimaan**: Dalam waktu maksimal **48 jam** sejak laporan diterima.
   - **Validasi & Triase**: Dalam waktu **5 hari kerja**, kami akan memberikan penilaian tingkat keparahan dan jadwal perbaikan.
   - **Rilis Perbaikan & Pengungkapan Terkoordinasi (*Coordinated Disclosure*)**: Setelah *patch* siap dan diuji, kami akan merilis pembaruan keamanan dan mencantumkan kredit apresiasi kepada pelapor (kecuali Anda memilih untuk tetap anonim).
