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

    // 2. Системные данные
    println(&format!("Случайное число из хоста: {}", random_u32()));
    println(&format!("Timestamp (sec): {}, (ms): {}", now_unix(), now_millis()));

    if let Some(path) = get_env("PATH") {
        println(&format!("PATH env (len {}): {}", path.len(), path));
    }

    println(&format!("PID процесса: {}", get_pid()));

    // 3. Сеть
    if let Ok(response) = http_get("http://httpbin.org/ip") {
        println(&format!("Ответ от сети:\n{}", response));
    } else {
        println("Ошибка HTTP-запроса");
    }

    // 4. Интерактивный ввод
    print("Введите ваше имя: ");
    let name = read_line();
    if !name.is_empty() {
        println(&format!("Привет, {}!", name));
    }

    // 5. Прогресс-бар
    print("Загрузка: ");
    for i in 1..=10 {
        print(&format!("\rЗагрузка: [{:<10}] {}%", "=".repeat(i), i * 10));
        sleep(100);
    }
    println("\nГотово!");

    exit(0);
}