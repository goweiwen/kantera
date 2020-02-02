use std::rc::Rc;
use std::cell::RefCell;
use gluten::{
    reader::{Reader, default_atom_reader},
    core::{eval, Env, macro_expand},
    StringPool
};
pub use gluten::data::*;
use crate::{
    image::Image,
    pixel::Rgba,
    render::Render,
    audio_render::AudioRender,
    path::{Path, Point},
    timed::Timed,
    v::{Vec2, Vec3}
};

pub struct Runtime(Env);

impl Runtime {
    pub fn new() -> Runtime {
        let reader = Reader::new(Box::new(atom_reader));
        let env = Env::new(Rc::new(RefCell::new(reader)));
        let mut rt = Runtime(env);
        init_runtime(&mut rt);
        rt
    }

    pub fn insert(&mut self, str: &str, val: Val) {
        let sym = self.0.reader().borrow_mut().intern(str);
        self.0.insert(sym, val);
    }

    pub fn get(&self, str: &str) -> Option<Val> {
        let sym = self.0.reader().borrow().try_intern(str)?;
        self.0.get(&sym)
    }

    pub fn re(&mut self, str: &str) -> Result<Val, String>{
        let forms = self.0.reader().borrow_mut().parse_top_level(str)?;
        let mut last_val = None;
        for form in forms {
            let form = macro_expand(&mut self.0, form);
            last_val = Some(eval(self.0.clone(), form));
        }
        last_val.ok_or("no form".to_string())
    }
}

fn atom_reader(sp: &mut StringPool, s: &str) -> Result<Val, String> {
    if let Ok(v) = s.parse::<i32>() {
        return Ok(r(v));
    }
    if let Ok(v) = s.parse::<f64>() {
        return Ok(r(v));
    }
    default_atom_reader(sp, s)
}

fn init_runtime(rt: &mut Runtime) {
    rt.insert("true", r(true));
    rt.insert("false", r(false));
    rt.insert("first", r(Box::new(|vec: Vec<Val>| {
        vec[0].clone()
    }) as MyFn));
    rt.insert("vec", r(Box::new(|vec: Vec<Val>| {
        r(vec)
    }) as MyFn));
    rt.insert("+", r(Box::new(|vec: Vec<Val>| -> Val {
        fn f<T: num_traits::Num + Copy + 'static>(vec: &Vec<Val>) -> Option<Val> {
            let mut acc = T::zero();
            for rv in vec.iter() {
                acc = acc + *rv.borrow().downcast_ref::<T>()?;
            }
            Some(r(acc))
        }
        f::<f64>(&vec).or_else(|| f::<i32>(&vec)).or_else(|| f::<Vec2<f64>>(&vec)).or_else(|| f::<Vec3<f64>>(&vec)).unwrap()
    }) as MyFn));
    rt.insert("-", r(Box::new(|vec: Vec<Val>| -> Val {
        fn f<T: num_traits::Num + Copy + 'static>(vec: &Vec<Val>) -> Option<Val> {
            let mut acc = *vec[0].borrow().downcast_ref::<T>()?;
            for rv in vec.iter().skip(1) {
                acc = acc - *rv.borrow().downcast_ref::<T>()?;
            }
            Some(r(acc))
        }
        f::<f64>(&vec).or_else(|| f::<i32>(&vec)).or_else(|| f::<Vec2<f64>>(&vec)).or_else(|| f::<Vec3<f64>>(&vec)).unwrap()
    }) as MyFn));
    rt.insert("*", r(Box::new(|vec: Vec<Val>| -> Val {
        fn f<T: num_traits::Num + Copy + 'static>(vec: &Vec<Val>) -> Option<Val> {
            let mut acc = T::one();
            for rv in vec.iter() {
                acc = acc * *rv.borrow().downcast_ref::<T>()?;
            }
            Some(r(acc))
        }
        f::<f64>(&vec).or_else(|| f::<i32>(&vec)).or_else(|| f::<Vec2<f64>>(&vec)).or_else(|| f::<Vec3<f64>>(&vec)).unwrap()
    }) as MyFn));
    rt.insert("/", r(Box::new(|vec: Vec<Val>| -> Val {
        fn f<T: num_traits::Num + Copy + 'static>(vec: &Vec<Val>) -> Option<Val> {
            let mut acc = *vec[0].borrow().downcast_ref::<T>()?;
            for rv in vec.iter().skip(1) {
                acc = acc / *rv.borrow().downcast_ref::<T>()?;
            }
            Some(r(acc))
        }
        f::<f64>(&vec).or_else(|| f::<i32>(&vec)).or_else(|| f::<Vec2<f64>>(&vec)).or_else(|| f::<Vec3<f64>>(&vec)).unwrap()
    }) as MyFn));
    rt.insert("stringify", r(Box::new(|vec: Vec<Val>| -> Val {
        fn f<T: std::fmt::Debug + 'static>(vec: &Vec<Val>) -> Option<Val> {
            Some(r(format!("{:?}", vec[0].borrow().downcast_ref::<T>()?)))
        }
        f::<String>(&vec).or_else(|| f::<Symbol>(&vec))
        .or_else(|| f::<f64>(&vec)).or_else(|| f::<i32>(&vec)).or_else(|| f::<Vec2<f64>>(&vec)).or_else(|| f::<Vec3<f64>>(&vec))
        .or_else(|| f::<Rgba>(&vec))
        .unwrap()
    }) as MyFn));
    rt.insert("rgb", r(Box::new(|vec: Vec<Val>| {
        use regex::Regex;
        if let Some(string) = vec[0].borrow().downcast_ref::<String>() {
            let re = Regex::new(r"#([\da-fA-F]{2})([\da-fA-F]{2})([\da-fA-F]{2})").unwrap();
            if let Some(cap) = re.captures(string) {
                fn f(s: &str) -> f64 {
                    let mut cs = s.chars();
                    (cs.next().unwrap().to_digit(16).unwrap() * 16 + cs.next().unwrap().to_digit(16).unwrap()) as f64 / 255.0
                }
                r(Rgba(
                    f(&cap[1]),
                    f(&cap[2]),
                    f(&cap[3]),
                    1.0,
                ))
            } else {
                panic!("invalid RGB string");
            }
        } else {
            r(Rgba(
                *vec[0].borrow().downcast_ref::<f64>().unwrap(),
                *vec[1].borrow().downcast_ref::<f64>().unwrap(),
                *vec[2].borrow().downcast_ref::<f64>().unwrap(),
                1.0
            ))
        }
    }) as MyFn));
    rt.insert("rgba", r(Box::new(|vec: Vec<Val>| {
        use regex::Regex;
        if let Some(string) = vec[0].borrow().downcast_ref::<String>() {
            let re = Regex::new(r"#([\da-fA-F]{2})([\da-fA-F]{2})([\da-fA-F]{2})([\da-fA-F]{2})").unwrap();
            if let Some(cap) = re.captures(string) {
                fn f(s: &str) -> f64 {
                    let mut cs = s.chars();
                    (cs.next().unwrap().to_digit(16).unwrap() * 16 + cs.next().unwrap().to_digit(16).unwrap()) as f64 / 255.0
                }
                r(Rgba(
                    f(&cap[1]),
                    f(&cap[2]),
                    f(&cap[3]),
                    f(&cap[4]),
                ))
            } else {
                panic!("invalid RGBA string");
            }
        } else {
            r(Rgba(
                *vec[0].borrow().downcast_ref::<f64>().unwrap(),
                *vec[1].borrow().downcast_ref::<f64>().unwrap(),
                *vec[2].borrow().downcast_ref::<f64>().unwrap(),
                *vec[3].borrow().downcast_ref::<f64>().unwrap()
            ))
        }
    }) as MyFn));
    rt.insert("plain", r(Box::new(|vec: Vec<Val>| {
        if let Some(p) = vec[0].borrow().downcast_ref::<Rgba>() {
            r(Rc::new(crate::renders::plain::Plain::new(*p)) as Rc<dyn Render<Rgba>>)
        } else if let Some(p) = vec[0].borrow().downcast_ref::<Path<Rgba>>() {
            r(Rc::new(crate::renders::plain::Plain::new(p.clone())) as Rc<dyn Render<Rgba>>)
        } else {
            panic!()
        }
    }) as MyFn));
    rt.insert("frame", r(Box::new(|vec: Vec<Val>| {
        use crate::renders::frame::{Frame, FrameType};
        let render = vec[0].borrow().downcast_ref::<Rc<dyn Render<Rgba>>>().unwrap().clone();
        let frame_type = match vec[1].borrow().downcast_ref::<Symbol>().unwrap().0.as_str() {
            "constant" => FrameType::Constant(*vec[2].borrow().downcast_ref::<Rgba>().unwrap()),
            "extend" => FrameType::Extend,
            "repeat" => FrameType::Repeat,
            "reflect" => FrameType::Reflect,
            _ => panic!("invalid frame_type")
        };
        r(Rc::new(Frame {render, frame_type}) as Rc<dyn Render<Rgba>>)
    }) as MyFn));
    rt.insert("sequence", r(Box::new(|vec: Vec<Val>| {
        let mut sequence = crate::renders::sequence::Sequence::new();
        for p in vec.into_iter() {
            let p = p.borrow().downcast_ref::<Vec<Val>>().unwrap().clone();
            let time = *p[0].borrow().downcast_ref::<f64>().unwrap();
            let restart = *p[1].borrow().downcast_ref::<bool>().unwrap();
            let render = p[2].borrow_mut().downcast_mut::<Rc<dyn Render<Rgba>>>().unwrap().clone();
            sequence = sequence.append(time, restart, render);
        }
        r(Rc::new(sequence) as Rc<dyn Render<Rgba>>)
    }) as MyFn));
    rt.insert("image_render", r(Box::new(|vec: Vec<Val>| {
        let image = vec[0].borrow().downcast_ref::<Rc<Image<Rgba>>>().unwrap().clone();
        let default = *vec[1].borrow().downcast_ref::<Rgba>().unwrap();
        r(Rc::new(crate::renders::image_render::ImageRender {
            image: image,
            sizing: crate::renders::image_render::Sizing::Contain,
            default: default,
            interpolation: crate::interpolation::Bilinear // TODO
        }) as Rc<dyn Render<Rgba>>)
    }) as MyFn));
    rt.insert("text_to_image", r(Box::new(|vec: Vec<Val>| {
        let string = vec[0].borrow().downcast_ref::<String>().unwrap().clone();
        let scale = *vec[1].borrow().downcast_ref::<f64>().unwrap();
        use crate::{text::{Font, render}};
        let font_path = "../IPAexfont00401/ipaexg.ttf"; // TODO
        let bytes = std::fs::read(font_path).unwrap();
        let font = Font::from_bytes(&bytes).unwrap();
        r(Rc::new(render(&font, scale as f32, &string).map(|v| Rgba(0.0, 0.0, 0.0, *v))))
    }) as MyFn));
    rt.insert("composite", r(Box::new(|vec: Vec<Val>| {
        use crate::renders::composite::{Composite, CompositeMode};
        let layers = vec.into_iter().map(|p| {
            let p = p.borrow().downcast_ref::<Vec<Val>>().unwrap().clone();
            let render = p[0].borrow_mut().downcast_mut::<Rc<dyn Render<Rgba>>>().unwrap().clone();
            let mode = p[1].borrow().downcast_ref::<Symbol>().unwrap().0.to_owned();
            let mode = match mode.as_str() {
                "none" => CompositeMode::None,
                "normal" => CompositeMode::Normal(Path::new(1.0)),
                _ => panic!("illegal CompositeMode")
            };
            (render, mode)
        }).collect();
        r(Rc::new(Composite {
            layers: layers
        }) as Rc<dyn Render<Rgba>>)
    }) as MyFn));
    fn vec_to_vec2<T: 'static + num_traits::Num + Copy + From<f64>>(val: &Val) -> Vec2<T> {
        let val = val.borrow();
        let vec = val.downcast_ref::<Vec<Val>>().unwrap();
        let a = *vec[0].borrow().downcast_ref::<T>().unwrap();
        let b = *vec[1].borrow().downcast_ref::<T>().unwrap();
        Vec2(a, b)
    }
    fn vec_to_vec3<T: 'static + num_traits::Num + Copy + From<f64>>(val: &Val) -> Vec3<T> {
        let val = val.borrow();
        let vec = val.downcast_ref::<Vec<Val>>().unwrap();
        let a = *vec[0].borrow().downcast_ref::<T>().unwrap();
        let b = *vec[1].borrow().downcast_ref::<T>().unwrap();
        let c = *vec[2].borrow().downcast_ref::<T>().unwrap();
        Vec3(a, b, c)
    }
    rt.insert("path", r(Box::new(|vec: Vec<Val>| {
        let mut it = vec.into_iter();
        fn build_path<T: 'static + Clone + crate::lerp::Lerp>(first_value: T, it: impl Iterator<Item = Val>, vectorize: &impl Fn(&Val) -> T) -> Val {
            let mut path = Path::new(first_value);
            for rp in it {
                let rp = rp.borrow();
                let p = rp.downcast_ref::<Vec<Val>>().unwrap();
                let d_time = *p[0].borrow().downcast_ref::<f64>().unwrap();
                let vec = vectorize(&p[1]);
                let point = match p[2].borrow().downcast_ref::<Symbol>().unwrap().0.as_str() {
                    "constant" => Point::Constant,
                    "linear" => Point::Linear,
                    "bezier" => Point::Bezier(vectorize(&p[3]), vectorize(&p[4])),
                    _ => panic!("invalid point type")
                };
                path = path.append(d_time, vec, point);
            }
            r(Rc::new(path) as Rc<dyn Timed<T>>)
        }
        if let Some(first_value) = it.next() {
            let v = first_value.borrow();
            if let Some(v) = v.downcast_ref::<f64>() {
                return build_path(*v, it, &|val| *val.borrow().downcast_ref::<f64>().unwrap());
            } else if let Some(v) = v.downcast_ref::<Rgba>() {
                return build_path(*v, it, &|val| *val.borrow().downcast_ref::<Rgba>().unwrap());
            } else if let Some(vec) = v.downcast_ref::<Vec<Val>>() {
                match vec.len() {
                    2 => {
                        return build_path(vec_to_vec2::<f64>(&first_value), it, &vec_to_vec2);
                    },
                    3 => {
                        return build_path(vec_to_vec3::<f64>(&first_value), it, &vec_to_vec3);
                    },
                    _ => {}
                }
            }
            panic!("illegal path arguments")
        } else {
            panic!("path requires at least one argument")
        }
    }) as MyFn));
    rt.insert("cycle", r(Box::new(|vec: Vec<Val>| {
        use crate::timed::Cycle;
        fn f<T: 'static>(vec: &Vec<Val>) -> Option<Val> {
            let timed = vec[0].borrow().downcast_ref::<Rc<dyn Timed<T>>>()?.clone();
            let duration = *vec[1].borrow().downcast_ref::<f64>().unwrap();
            Some(r(Rc::new(Cycle::new(timed, duration)) as Rc<dyn Timed<T>>))
        }
        f::<f64>(&vec).or_else(|| f::<Vec2<f64>>(&vec)).or_else(|| f::<Vec3<f64>>(&vec)).unwrap()
    }) as MyFn));
    rt.insert("sin", r(Box::new(|vec: Vec<Val>| {
        use crate::timed::Sine;
        fn f<T: 'static + Clone + Timed<f64>>(vec: &Vec<Val>) -> Option<Val> {
            let initial_phase = *vec[0].borrow().downcast_ref::<f64>().unwrap();
            let frequency = vec[1].borrow().downcast_ref::<f64>().unwrap().clone();
            let amplitude = vec[2].borrow().downcast_ref::<T>().unwrap().clone();
            Some(r(Rc::new(Sine::new(initial_phase, frequency, amplitude)) as Rc<dyn Timed<f64>>))
        }
        f::<f64>(&vec).or_else(|| f::<Rc<dyn Timed<f64>>>(&vec)).unwrap()
    }) as MyFn));
    rt.insert("transform", r(Box::new(|vec: Vec<Val>| {
        use crate::{renders::transform::{Transform, timed_to_transformer}};
        let render = vec[0].borrow_mut().downcast_mut::<Rc<dyn Render<Rgba>>>().unwrap().clone();
        fn get_timed_vec2(val: &Val) -> Rc<dyn Timed<Vec2<f64>>> {
            if let Some(timed) = val.borrow().downcast_ref::<Rc<dyn Timed<Vec2<f64>>>>() {
                timed.clone()
            } else {
                let val = val.borrow();
                let v = val.downcast_ref::<Vec<Val>>().unwrap();
                let a = *v[0].borrow().downcast_ref::<f64>().unwrap();
                let b = *v[1].borrow().downcast_ref::<f64>().unwrap();
                Rc::new(Vec2(a, b))
            }
        }
        fn get_timed_f64(val: &Val) -> Rc<dyn Timed<f64>> {
            if let Some(timed) = val.borrow().downcast_ref::<Rc<dyn Timed<f64>>>() {
                timed.clone()
            } else {
                Rc::new(val.borrow().downcast_ref::<f64>().unwrap().clone())
            }
        }
        let translation_timed = get_timed_vec2(&vec[1]);
        let scale_timed = get_timed_vec2(&vec[2]);
        let rotation_timed = get_timed_f64(&vec[3]);
        r(Rc::new(Transform::new(
            render,
            timed_to_transformer(translation_timed, scale_timed, rotation_timed)
        )) as Rc<dyn Render<Rgba>>)
    }) as MyFn));
    rt.insert("test_audio", r(Box::new(|_vec: Vec<Val>| {
        use crate::audio_renders::{note::Note, sequencer::Sequencer};
        fn note(dur: f64, nn: i32, vel: f64, pan: f64) -> Box<dyn AudioRender> {
            Box::new(Note {
                frequency: 440.0 * 2.0f64.powf((nn - 69) as f64 / 12.0),
                duration: dur,
                gain: vel,
                pan: pan
            })
        }
        //r(Rc::new(note(1.0, 60, 0.3, 1.0)) as Rc<dyn AudioRender>)
        r(Rc::new(Sequencer::new()
            .append(0.00, note(1.0, 60, 0.2, -1.0))
            .append(1.00, note(1.0, 64, 0.2, -1.0))
            .append(2.00, note(1.0, 62, 0.2, -1.0))
            .append(3.00, note(1.0, 67, 0.2, -1.0))
            .append(4.00, note(1.0, 60, 0.2, 1.0))
            .append(5.00, note(1.0, 64, 0.2, 1.0))
            .append(6.00, note(1.0, 62, 0.2, 1.0))
            .append(7.00, note(1.0, 67, 0.2, 1.0))
            .append(0.00, note(0.25, 72, 0.1, 0.0))
            .append(0.50, note(0.25, 72, 0.1, 0.0))
            .append(1.00, note(0.25, 72, 0.1, 0.0))
            .append(1.50, note(0.25, 72, 0.1, 0.0))
            .append(2.00, note(0.25, 72, 0.1, 0.0))
            .append(2.50, note(0.25, 72, 0.1, 0.0))
            .append(2.00, note(0.25, 72, 0.1, 0.0))
            .append(2.50, note(0.25, 72, 0.1, 0.0))
            .append(3.00, note(0.25, 72, 0.1, 0.0))
            .append(3.50, note(0.50, 74, 0.1, 0.0))
            .append(4.00, note(0.25, 72, 0.1, 0.0))
            .append(4.50, note(0.25, 72, 0.1, 0.0))
            .append(5.00, note(0.25, 72, 0.1, 0.0))
            .append(5.50, note(0.25, 72, 0.1, 0.0))
            .append(6.00, note(0.25, 72, 0.1, 0.0))
            .append(6.50, note(0.25, 72, 0.1, 0.0))
            .append(7.00, note(0.25, 72, 0.1, 0.0))
            .append(7.50, note(0.50, 74, 0.1, 0.0))) as Rc<dyn AudioRender>)
    }) as MyFn));
    rt.insert("import_image", r(Box::new(|vec: Vec<Val>| {
        let filepath = vec[0].borrow().downcast_ref::<String>().unwrap().clone();
        r(Rc::new(crate::image_import::load_image(&filepath)))
    }) as MyFn));
    #[cfg(feature = "ffmpeg")]
    rt.insert("import_audio", r(Box::new(|vec: Vec<Val>| {
        let filepath = vec[0].borrow().downcast_ref::<String>().unwrap().clone();
        r(Rc::new(crate::ffmpeg::import_audio(&filepath)))
    }) as MyFn));
}
