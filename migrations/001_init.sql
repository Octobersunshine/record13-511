-- 员工表：存储正式员工信息
CREATE TABLE IF NOT EXISTS employees (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    employee_no TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    department TEXT NOT NULL,
    card_no TEXT NOT NULL UNIQUE,
    position TEXT,
    phone TEXT,
    status INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- 访客表：存储临时访客信息
CREATE TABLE IF NOT EXISTS visitors (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    visitor_no TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    id_card_no TEXT,
    company TEXT,
    purpose TEXT NOT NULL,
    visited_employee_id INTEGER,
    card_no TEXT NOT NULL UNIQUE,
    valid_from TEXT NOT NULL,
    valid_to TEXT NOT NULL,
    status INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (visited_employee_id) REFERENCES employees(id)
);

-- 通行记录表：存储所有刷卡通行日志
CREATE TABLE IF NOT EXISTS access_records (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    record_no TEXT NOT NULL UNIQUE,
    card_no TEXT NOT NULL,
    person_type TEXT NOT NULL CHECK (person_type IN ('employee', 'visitor', 'unknown')),
    person_id INTEGER,
    person_name TEXT,
    department TEXT,
    door_no TEXT NOT NULL,
    direction TEXT NOT NULL CHECK (direction IN ('in', 'out')),
    access_result TEXT NOT NULL CHECK (access_result IN ('allowed', 'denied')),
    deny_reason TEXT,
    swiped_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- 索引优化
CREATE INDEX IF NOT EXISTS idx_access_records_card_no ON access_records(card_no);
CREATE INDEX IF NOT EXISTS idx_access_records_swiped_at ON access_records(swiped_at);
CREATE INDEX IF NOT EXISTS idx_access_records_person_type ON access_records(person_type);
CREATE INDEX IF NOT EXISTS idx_employees_card_no ON employees(card_no);
CREATE INDEX IF NOT EXISTS idx_visitors_card_no ON visitors(card_no);

-- 插入示例员工数据
INSERT OR IGNORE INTO employees (employee_no, name, department, card_no, position, phone, status) VALUES
('EMP001', '张三', '技术部', 'CARD-EMP-001', '高级工程师', '13800138001', 1),
('EMP002', '李四', '市场部', 'CARD-EMP-002', '市场经理', '13800138002', 1),
('EMP003', '王五', '人事部', 'CARD-EMP-003', 'HR专员', '13800138003', 1);

-- 插入示例访客数据
INSERT OR IGNORE INTO visitors (visitor_no, name, id_card_no, company, purpose, visited_employee_id, card_no, valid_from, valid_to, status) VALUES
('VIS001', '赵六', '110101199001011234', 'ABC科技公司', '商务洽谈', 1, 'CARD-VIS-001', datetime('now', '-1 day'), datetime('now', '+7 day'), 1),
('VIS002', '钱七', '110101199203034567', 'XYZ供应商', '项目交付', 2, 'CARD-VIS-002', datetime('now'), datetime('now', '+1 day'), 1);
