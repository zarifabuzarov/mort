mod sdk;
use sdk::*;

#[no_mangle]
pub extern "C" fn start() {
    let file_path = "test_run.txt";

    // 1. Работа с файлами
    write_file(file_path, "1. Первая строка (перезапись)\n");
    append_file(file_path, "2. Вторая строка (дозапись)\n");

    if let Ok(content) = read_file(file_path) {
        println("--- Содержимое файла ---");
        sleep(2000);
        print(&content);
    } else {
        println("Ошибка при чтении файла!");
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