# MQTT RUSTICO - Fra2ManEze

[![MQTT](docs/mqtt.png)](https://mqtt.org/)
[![Rust](docs/rust.png)](https://www.rust-lang.org/)

Introducción
------
En este repositorio se encuentra el código fuente de la implementación de un servidor y un cliente para el protocolo MQTT 3.1.1, realizado en Rust para la materia Taller de Programación I (75.42/95.08) cátedra Deymonnaz.

Integrantes
------
- Bilbao, Manuel Iñaki
- Vilardo, Ezequiel
- Primerano Lomba, Franco Alejandro
- Ferrer Vieyras, Franco

¿Qué es MQTT?
------
MQTT es un protocolo de mensajería basado en el patrón de comunicación publisher-suscriber y la arquitectura cliente-servidor. Dentro de sus principales características se puede destacar que es un protocolo liviano, simple y sencillo de implementar. Es un protocolo de capa de aplicación binario construido sobre TCP/IP, lo cual lo convierte en una forma de comunicarse sumamente eficiente con un overhead mínimo en la cantidad de paquetes que se envían a través de la red, a diferencia de otros protocolos de capa de aplicación, como por ejemplo HTTP.

Compilación y ejecución
------

### Servidor

En caso de no tener instalado Rust, se debe instalar. Por ejemplo, para distribuciones basadas en Debian:

```
apt update && apt install curl build-essential
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Una vez instalado, ya se puede compilar el servidor:

```
cd server
cargo build
```

Y para ejecutarlo:

```
cargo run src/config.txt
```

En el archivo `server/src/config.txt` se encuentran las configuraciones del mismo.

### Cliente

Nuevamente, se deberá tener instalado Rust. En caso de no tenerlo, ver la sección [Servidor](#servidor).

Además, se deben tener instaladas las dependencias `pkg-config` y [GTK3](https://www.gtk.org/docs/installations/). Para distribuciones basadas en Debian:

```
apt update && apt install pkg-config libgtk-3-dev
```

Para compilar, se ejecuta:

```
cd client
cargo build
```

Y para ejecutar, simplemente:

```
cargo run
```

### Documentación

Se puede generar la documentación del código de cada programa (cliente y servidor) por separado mediante el comando:

```
cd [server|client]
cargo doc
```

Se puede encontrar una versión de la documentación del servidor en el siguiente [enlace](https://manuelbilbao.github.io/Fra2ManEze/).