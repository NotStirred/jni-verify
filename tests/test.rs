use jni::objects::JObject;
use jni::sys::jobject;
use jni_verify::verify_signature;

use jni::objects::JClass;
use jni::objects::JString;
use jni::sys::jfloat;
use jni::sys::jint;
use jni::sys::jstring;
use jni::JNIEnv;

#[verify_signature("foo", "(Lsome.package.Foo;asdI)Ljava.lang.Foo;")]
#[no_mangle]
pub extern "system" fn Java_World_foo<'local>(
    mut _env: JNIEnv<'local>,
    _class: JClass<'local>,
    _input: JObject<'local>,
    _i: jint,
) -> jfloat {
    unimplemented!()
}

#[verify_signature("foo2", "(Ljava.lang.String;F)Ljava.lang.String;")]
#[no_mangle]
pub extern "system" fn Java_Test_foo2<'local>(
    mut _env: JNIEnv<'local>,
    _class: JClass<'local>,
    _input: JString<'local>,
    _foo: jfloat,
) -> jstring {
    unimplemented!()
}

#[verify_signature("foo3", "()Ljava.lang.String;")]
#[no_mangle]
pub extern "system" fn Java_Test_foo3<'local>(
    mut _env: JNIEnv<'local>,
    _class: JClass<'local>,
) -> jstring {
    unimplemented!()
}

#[verify_signature("foo4_123_d____", "(Ljava.lang.String;F)V")]
#[no_mangle]
pub extern "system" fn Java_Test_foo4_123_d____<'local>(
    mut _env: JNIEnv<'local>,
    _class: JClass<'local>,
    _input: JString<'local>,
    _foo: jfloat,
) {
    unimplemented!()
}
