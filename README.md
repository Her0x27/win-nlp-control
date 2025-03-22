# win-nlp-control

```tree
.
winui-automation/
├── Cargo.toml         # Файл манифеста Cargo
├── src/
│   ├── main.rs          # Главный файл приложения (Web API)
│   ├── core/            # Основные компоненты приложения
│   │   ├── config.rs      # Работа с конфигурацией
│   │   ├── language.rs    # Работа с языковыми файлами
│   │   ├── intent.rs      # Определение намерений и действий
│   │   ├── nlp.rs         # Обработка естественного языка (NLP)
│   ├── platform/       # Абстракции для конкретной платформы
│   │   ├── windows/     # Реализация для Windows
│   │   │   ├── winapi.rs  # Обёртки вокруг windows-sys
│   │   │   ├── controller.rs # Управление WinUI элементами
│   │   ├── mock/        # Мок-реализация (для тестирования и разработки на других платформах)
│   │   │   └── controller.rs
│   ├── task/            # Управление задачами и их выполнением
│   │   ├── model.rs      # Определение структур данных для задач
│   │   ├── scheduler.rs  # Планировщик задач
│   │   ├── executor.rs   # Модуль для запуска тасков
│   ├── webapi/          # Web API endpoints
│   │   ├── handlers.rs    # Обработчики Web API запросов
│   │   ├── models.rs      # DTO для webapi
├── assets/           # Разные ресурсы
│   ├── natural.config    # Файл конфигурации (JSON)
│   ├── lang/           # Языковые файлы
│   │   └── ru.json        # Файл языковых ресурсов (JSON, русский)
```
