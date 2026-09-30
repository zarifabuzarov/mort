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

    // 2. Системные данные
    println(&format!("Случайное число из хоста: {}", random_u32()));
    println(&format!("Timestamp (sec): {}, (ms): {}", now_unix(), now_millis()));

    if let Some(path) = get_env("PATH") {
        let preview = if path.len() > 100 { &path[..100] } else { &path };
        println(&format!("PATH env (len {}): {}...", path.len(), preview));
    } else {
        println("Переменная PATH не найдена");
    }

    println(&format!("PID процесса: {}", get_pid()));

    // 3. Сеть
    if let Ok(response) = http_get("http://httpbin.org/ip") {
        println(&format!("Ответ от сети:\n{:?}", response));
    } else {
        println("Ошибка HTTP-запроса");
    }

    // 4. Интерактивный ввод
    print("Введите ваше имя: ");
    let name = read_line();
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
    let Ok(src_bytes) = sdk::fs::read("input.jpg") else {
        println("Ошибка: не удалось открыть input.jpg");
        return;
    };

    // 2. Создаем трансформацию красиво
    let transform = ImageTransform::new_resize(300, 300).with_rotate(90);

    // 3. Обрабатываем и сохраняем одной строчкой
    if let Some(processed) = process_image(&src_bytes, &transform) {
        if sdk::fs::write("output_thumbnail.jpg", &processed).is_ok() {
            println(&format!("Готово! Картинка урезана до {} байт", processed.len()));
        }
    }

    println("------------------------------------");

    exit(0);
}