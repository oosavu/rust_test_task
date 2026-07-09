use std::ptr::null;
use std::marker::PhantomPinned;

/// ВАЖНО: все задания выполнять не обязательно. Что получится то получится сделать.

/// Задание 1
/// Почему фунция example1 зависает?
/// ответ: первая feature вызывает try_recv, не отдавая управление tokio. А так как runtime запущен в одном потоке,
/// то вторая feature не выполняется.
/// решение1: увеличить колличество потоков что бы было место под вторую feature,
/// решение2: либо использовать try_recv в цикле с yield_now,
/// решение3 (кажется наиболее правильное): либо использовать recv вместо try_recv
fn example1() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1) // можно .worker_threads(2)
        .build()
        .unwrap();
    let (sd, mut rc) = tokio::sync::mpsc::unbounded_channel();

    println!("execution started");
    let a1 = async move {
        loop {
            if let Some(p) = rc.recv().await {
                println!("{}", p);
                
                break;
            }
        }
    };
    let h1 = rt.spawn(a1);

    let a2 = async move {
        let _ = sd.send("message");
    };
    let h2 = rt.spawn(a2);
    while !(h1.is_finished() || h2.is_finished()) {}

    println!("execution completed");
}

#[derive(Clone)]
struct Example2Struct {
    value: u64,
    ptr: *const u64,
}

// неправильный вариант реализации Clone, NRVO не всегда срабатывает
// impl Clone for Example2Struct {
//     fn clone(&self) -> Self {
//         let mut ans = Example2Struct {
//             value: self.value,
//             ptr: null(),
//         };
//         ans.ptr = &ans.value as *const u64;
//         ans

//     }
// }

/// Задание 2
/// Какое число тут будет распечатано 32 64 или 128 и почему?
/// выведет 64, так как t2.ptr указывает на t1.value, а t1.value не изменялся после клонирования
/// 
/// 
/// решение 1 (не верное): сначала подумал что достаточно просто правильно реализовать Clone, 
/// но не помогло, в дебаге не работает NRVO
/// 
/// решение 2 (костыльное): просто каждый раз руками присваивать ptr на value
/// 
/// Правильное решение наверное такое: запинить структуру, но тогда ее интерфейс изменится.
/// позже попытаюсь реализовать.
fn example2() {

    let num = 32;

    let mut t1 = Example2Struct {
        value: 64,
        ptr: &num,
    };

    t1.ptr = &t1.value;

    let mut t2 = t1.clone();

    drop(t1);

    t2.ptr = &t2.value;

    t2.value = 128;
    

    unsafe {
        println!("{}", t2.ptr.read());
    }

    println!("execution completed");
}

/// Задание 3
/// Почему время исполнения всех пяти заполнений векторов разное (под linux)?
/// 
/// 
/// ответ: поправил четвертый вариант, так как он не менял значения вектора, а просто менял локальную переменную.
/// cargo run --release
// execution time 15383291
// execution time 11304167
// execution time 5430208
// execution time 42
// execution time 42
// cargo run --debug
// execution time 132575666
// execution time 103532208
// execution time 32253416
// execution time 26145833
// execution time 6917
fn example3() {
    let capacity = 10000000u64;


    // тут хуже всего. реаллокация + ручное заполнение каждого элемента.
    let start_time = std::time::Instant::now();
    let mut my_vec1 = Vec::new();
    for i in 0u64..capacity {
        my_vec1.insert(i as usize, i);
    }
    println!(
        "execution time {}",
        (std::time::Instant::now() - start_time).as_nanos()
    );

    // уже лучше. убоали реаллокацию.
    let start_time = std::time::Instant::now();
    let mut my_vec2 = Vec::with_capacity(capacity as usize);
    for i in 0u64..capacity {
        my_vec2.insert(i as usize, i);
    }
    println!(
        "execution time {}",
        (std::time::Instant::now() - start_time).as_nanos()
    );

    // могу ответить не точно, но скорее всего здесь срабатывает векторизация записи памяти, 
    // надо смотреть внутрь макроса vec![]. 
    let start_time = std::time::Instant::now();
    let mut my_vec3 = vec![6u64; capacity as usize];
    println!(
        "execution time {}",
        (std::time::Instant::now() - start_time).as_nanos()
    );

    // здесь скорее всего тоже векторизация, так как значение одно и то же
    let start_time = std::time::Instant::now();
    for elem in &mut my_vec3 {
        *elem = 7u64;
    }
    println!(
        "execution time {}",
        (std::time::Instant::now() - start_time).as_nanos()
    );
    // в релизе видимо вообще этот цикл выкидывается, если 
    // эту строчку раскомментировать, то время станет сравнимым с предыдущим
    //print!("{}", my_vec3[0]); 
    

    // здесь макрос vec![] разворачивается просто в выделение памяти.
    // есть возможность у системного аллокатора попросить именно зануленную память
    // но при обращении в дальнейшем будет происходить уже реальное выделение физической памяти,
    // там будет неявный memset 
    let start_time = std::time::Instant::now();
    let my_vec4 = vec![0u64; capacity as usize];
    println!(
        "execution time {}",
        (std::time::Instant::now() - start_time).as_nanos()
    );

    println!("execution completed");
}

/// Задание 4
/// Почему такая разница во времени выполнения example4_async_mutex и example4_std_mutex?
/// 
/// ответ: std::mutex быстрее так-как он реализован через futex, который прежде чем сделать вызов через
/// libc и создать mutex сначала делает несколько итераций spin_lock-а
/// В tokio mutex такого не происходит, он честно взаимодействует с tokio runtime
/// 
/// оригинальный код
/// execution time 3959287583
/// execution time 92004500
/// 
/// заменил условие остановки
/// execution time 3290117584
/// execution time 157927292
/// 
/// заменил плохой код в циклах (там значение не менялось, только читалось):
/// execution time 3282834292
/// execution time 172313125
/// 
/// сделал три потока для рантайма:
/// execution time 3276831000
/// execution time 206903875
/// 
async fn example4_async_mutex(tokio_protected_value: std::sync::Arc<tokio::sync::Mutex<u64>>) {
    for _ in 0..1000000 {
        //wtf?
        //let mut value = *tokio_protected_value.clone().lock().await;
        //value = 4;
        let mut value = tokio_protected_value.lock().await;
        *value = 4;
    }
}

async fn example4_std_mutex(protected_value: std::sync::Arc<std::sync::Mutex<u64>>) {
    for _ in 0..1000000 {
        //wtf?
        //let mut value = *protected_value.clone().lock().unwrap();
        //value = 4;
        let mut value = protected_value.lock().unwrap();
        *value = 4
    }
}

fn example4() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        //.worker_threads(2) лучше три потока 
        .worker_threads(3)
        .build()
        .unwrap();

    let mut tokio_protected_value = std::sync::Arc::new(tokio::sync::Mutex::new(0u64));

    let start_time = std::time::Instant::now();
    let h1 = rt.spawn(example4_async_mutex(tokio_protected_value.clone()));
    let h2 = rt.spawn(example4_async_mutex(tokio_protected_value.clone()));
    let h3 = rt.spawn(example4_async_mutex(tokio_protected_value.clone()));

    // странное условие, ждем пока только один завершится? + нагружаем процессор в бесконечном цикле
    // while !(h1.is_finished() || h2.is_finished() || h3.is_finished()) {}
    rt.block_on(async {
        let _ = tokio::join!(h1, h2, h3);
    });
    println!(
        "execution time {}",
        (std::time::Instant::now() - start_time).as_nanos()
    );

    let protected_value = std::sync::Arc::new(std::sync::Mutex::new(0u64));

    let start_time = std::time::Instant::now();
    let h1 = rt.spawn(example4_std_mutex(protected_value.clone()));
    let h2 = rt.spawn(example4_std_mutex(protected_value.clone()));
    let h3 = rt.spawn(example4_std_mutex(protected_value.clone()));

    //while !(h1.is_finished() || h2.is_finished() || h3.is_finished()) {}
    rt.block_on(async {
        let _ = tokio::join!(h1, h2, h3);
    });
    println!(
        "execution time {}",
        (std::time::Instant::now() - start_time).as_nanos()
    );

    println!("execution completed");
}

/// Задание 5
/// В чем ошибка дизайна? Каких тестов не хватает? Есть ли лишние тесты?
mod example5 {
    pub struct Triangle {
        pub a: (f32, f32),
        pub b: (f32, f32),
        pub c: (f32, f32),
        area: Option<f32>,
        perimeter: Option<f32>,
    }

    impl Triangle {
        //calculate area which is a positive number
        pub fn area(&mut self) -> f32 {
            if let Some(area) = self.area {
                area
            } else {
                self.area = Some(f32::abs(
                    (1f32 / 2f32) * (self.a.0 - self.c.0) * (self.b.1 - self.c.1)
                        - (self.b.0 - self.c.0) * (self.a.1 - self.c.1),
                ));
                self.area.unwrap()
            }
        }

        fn dist(a: (f32, f32), b: (f32, f32)) -> f32 {
            f32::sqrt((b.0 - a.0) * (b.0 - a.0) + (b.1 - a.1) * (b.1 - a.1))
        }

        //calculate perimeter which is a positive number
        pub fn perimeter(&mut self) -> f32 {
            if let Some(perimeter) = self.perimeter {
                return perimeter;
            } else {
                self.perimeter = Some(
                    Triangle::dist(self.a, self.b)
                        + Triangle::dist(self.b, self.c)
                        + Triangle::dist(self.c, self.a),
                );
                self.perimeter.unwrap()
            }
        }

        //new makes no guarantee for a specific values of a,b,c,area,perimeter at initialization
        pub fn new() -> Triangle {
            Triangle {
                a: (0f32, 0f32),
                b: (0f32, 0f32),
                c: (0f32, 0f32),
                area: None,
                perimeter: None,
            }
        }
    }
}

#[cfg(test)]
mod example5_tests {
    use super::example5::Triangle;

    #[test]
    fn test_area() {
        let mut t = Triangle::new();

        t.a = (0f32, 0f32);
        t.b = (0f32, 0f32);
        t.c = (0f32, 0f32);

        assert!(t.area() == 0f32);

        let mut t = Triangle::new();

        t.a = (0f32, 0f32);
        t.b = (0f32, 1f32);
        t.c = (1f32, 0f32);

        assert!(t.area() == 0.5);

        let mut t = Triangle::new();

        t.a = (0f32, 0f32);
        t.b = (0f32, 1000f32);
        t.c = (1000f32, 0f32);

        println!("{}",t.area());
    }

    #[test]
    fn test_perimeter() {
        let mut t = Triangle::new();

        t.a = (0f32, 0f32);
        t.b = (0f32, 0f32);
        t.c = (0f32, 0f32);

        assert!(t.perimeter() == 0f32);

        let mut t = Triangle::new();

        t.a = (0f32, 0f32);
        t.b = (0f32, 1f32);
        t.c = (1f32, 0f32);

        assert!(t.perimeter() == 2f32 + f32::sqrt(2f32));
    }
}

fn main() {
    example4();

}