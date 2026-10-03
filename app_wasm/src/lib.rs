mod sdk;
use sdk::*;

#[no_mangle]
pub extern "C" fn start() {
    if let Ok(mut file) = FileHandle::create_write("test_run.txt") {
        file.write(b"Chunk 1...\n").ok();
        file.write(b"Chunk 2...\n").ok();
    }

    // Чтение кусочками по 8 байт
    if let Ok(mut file) = FileHandle::open_read("test_run.txt") {
        let mut buf = [0u8; 8];
        while let Ok(bytes_read) = file.read(&mut buf) {
            if bytes_read == 0 { break; } // EOF
            let text = String::from_utf8_lossy(&buf[..bytes_read]);
            print(&text);
        }
    }

    // 1. Узнать размер файла
    if let Some(sz) = file_size("test_run.txt") {
        println(&format!("Размер test_run.txt: {} байт", sz));
    }

    // 2. Использовать seek
    if let Ok(mut file) = FileHandle::open_read("test_run.txt") {
        file.seek(9).ok(); // пропустить первые 9 байт
        let mut buf = [0u8; 8];
        if let Ok(n) = file.read(&mut buf) {
            println(&format!("Смещенное чтение: {}", String::from_utf8_lossy(&buf[..n])));
        }
    }

    // 3. Переименовать файл
    rename("test_run.txt", "test_run_renamed.txt");

    // Тест CWD
    if let Ok(cwd) = current_dir() {
        println(&format!("Текущая директория: {}", cwd));
    }

    // Тест Stat
    if let Some(st) = stat("test_run_renamed.txt") {
        println(&format!("Stat: size={} bytes, modified={}s, mode={:o}, readonly={}",
            st.size, st.modified_sec, st.permissions, st.is_readonly == 1));
    }

    // 2. Системные данные
    println(&format!("Случайное число из хоста: {}", random_u32()));
    println(&format!("Timestamp (sec): {}, (ms): {}, (ns): {}", now_unix(), now_millis(), monotonic_nanos()));

    if let Some(path) = get_env("PATH") {
        let preview = if path.len() > 100 { &path[..100] } else { &path };
        println(&format!("PATH env (len {}): {}...", path.len(), preview));
    } else {
        println("Переменная PATH не найдена");
    }

    println!("PID: {}", get_pid());
    println!("CPU cores: {}", cpu_count());
    println!("RAM Total: {} MB", total_memory() / 1024 / 1024);
    println!("RAM Free: {} MB", free_memory() / 1024 / 1024);

    println!("CLI Args: {:?}", args());

    // 3. Сеть
    if let Ok(response) = http_get("http://httpbin.org/ip") {
        println(&format!("Ответ от сети:\n{:?}", response));
    } else {
        println("Ошибка HTTP-запроса");
    }

    // ТЕСТ UDP: Отправляем эхо на публичный сервер
    if let Ok(udp) = UdpSocket::bind("0.0.0.0:0") {
        let msg = b"Ping WASM UDP";
        if let Ok(sent) = udp.send_to(msg, "8.8.8.8:53") {
            println(&format!("UDP отправлено: {} байт на 8.8.8.8:53", sent));
        }
    }

    // ТЕСТ TCP: Пробуем подключиться к Google
    if let Ok(mut tcp) = TcpStream::connect("google.com:80") {
        tcp.send(b"GET / HTTP/1.1\r\nHost: google.com\r\nConnection: close\r\n\r\n").ok();
        let mut buf = [0u8; 128];
        if let Ok(read) = tcp.recv(&mut buf) {
            println(&format!("TCP Ответ (первые байты):\n{}", String::from_utf8_lossy(&buf[..read])));
        }
    }

    // 4. Интерактивный ввод
    print("Введите ваше имя: ");
    let name = "Zaga";//read_line();
    if !name.is_empty() {
        println(&format!("Привет, {}!", name));
    }

    // --- Тест Криптографии ---
    println("--- Crypto Test ---");

    // 1. Случайные байты
    let mut rnd_buf = [0u8; 16];
    if random_bytes(&mut rnd_buf) {
        println(&format!("Random bytes (hex): {:02x?}", rnd_buf));
    }

    // 2. SHA-256
    let data = b"Hello WASM Crypto!";
    if let Some(hash) = sha256(data) {
        let hex_str: String = hash.iter().map(|b| format!("{:02x}", b)).collect();
        println(&format!("SHA-256 ('Hello WASM Crypto!'): {}", hex_str));
    }

    // 3. SHA-512
    if let Some(hash) = sha512(data) {
        let hex_str: String = hash.iter().map(|b| format!("{:02x}", b)).collect();
        println(&format!("SHA-512 (len {}): {}...", hash.len(), &hex_str[..32]));
    }
    println("-------------------");

    // 5. Прогресс-бар
    print("Загрузка: ");
    for i in 1..=10 {
        print(&format!("\rЗагрузка: [{:<10}] {}%", "=".repeat(i), i * 10));
        sleep(100);
    }
    println("\nГотово!");

    println("--- Тест обработки медиа в Госте ---");

    // 1. Читаем одной строчкой через SDK
    let Ok(src_bytes) = sdk::read("input.jpg") else {
        println("Ошибка: не удалось открыть input.jpg");
        return;
    };

    // 2. Создаем трансформацию красиво
    let transform = ImageTransform::new_resize(200, 400).with_rotate(90);

    // 3. Обрабатываем и сохраняем одной строчкой
    if let Some(processed) = process_image(&src_bytes, &transform) {
        if sdk::write("output_thumbnail.jpg", &processed).is_ok() {
            println(&format!("Готово! Картинка урезана до {} байт", processed.len()));
        }
    }

    println("------------------------------------");

    println(&format!("Timestamp (sec): {}, (ms): {}, (ns): {}", now_unix(), now_millis(), monotonic_nanos()));

    exit(0);
}