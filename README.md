# Trabajo Practico 1: CoffeeGPT - 1we Cautrimestre 2023

## Iniciar el programa
```cargo run <path_archivo>```
`<path_arhivo>` es el path donde se encuentra el archivo que se utilizará para la ejecución. 
Cada línea de este archivo representa un pedido.

Hay un archivo bien simple [`pedidos.txt`] con varios pedidos que piden pocos ingredientes para probar conceptualmente el programa.
Bajo el directiorio [`src/tests/`] hay más archivos que especifican en su nombre los casos de uso que se estan testeando. Se pueden utilizar los mismos para correr el programa. Algunos de ellos fueron utilizados también para los tests unitarios.

## Pedidos
Los pedidos deben tener la información de las cantidades de ingredientes separadas por comas.
Los ingredientes son café molido, agua caliente, cacao y espuma de leche. Y el formato es el siguiente:
```<cafe_molido>,<agua_caliente>,<cacao>,<espuma_de_leche>```
Todos ellos son numeros naturales.

A los pedidos se les asignara automáticamente un identificador que corresponderá con la línea en la que se encuentran detallados.
Comenzando por el 0.

Si el parseo de algún pedido falla por algún dato invalido o algún otro error, se imprimira el error causante pero la ejecución continuara salteandose ese pedido.

## Modulos
### Cafetera (`CoffeMaker`)
La cafetera tiene un contenedor para cada uno de los ingredientes que se pueden solicitar en un pedido.
* Un dispensador de café que contiene café molido y granos de café.
* Un dispensador de agua caliente que contiene agua caliente y una conexión a la red para poder calentar.
* Un dispensador de cacao.
* Un dispensador de leche, que contiene espuma de leche y leche fría.

Cuando llega un pedido se va a intentar servir el ingrediente que tenga el contenedor disponible. Para ello se va probando
por cada uno de los contenedores a ver si estan disponibles y si tienen la cantidad suficiente para servir. 
Una vez que el pedido consigue el contenedor, sirve el ingrediente -esto tarda un tiempo definido para simular una acción real-,
y actualiza las cantidades de disponibilidad de ingredientes y de ingredientes utilizados para mantener las estadísticas.

Cada vez que el pedido termina de servirse algún ingrediente, se chequea si el mismo ya esta finalizado.
Cuando ya tiene todos los ingredientes servidos se finaliza la ejecución de ese hilo.

Por otra parte, hay 3 hilos corriendo, uno para cada contenedor menos el de cacao. Esots se fijan cuando necesitan recargar ingredientes. Sera explicado mas adelante.

Una vez que todos los pedidos son finalizados, se envía una señal de apagado a estos 3 contenedores y al hilo que imprime las estadísticas.

### Contenedor de café (`CoffeeContainer`)
El contenedor de café es el encargado de llevar un registro de sus ingredientes disponibles y utilizados:
* `ground_coffee_container`: Es la cantidad de café molido disponible para el uso en un pedido.
* `coffee_grains_container`: Es la cantidad de granos de café disponibles para ser utilizados para la reposición de café molido.
* `ground_coffee_used`:  Cantidad de café molido utilizado. Se va a usar para las estadísticas.
* `coffee_grains_used`: Cantidad de granos de café utilizados. Se va a usar para las estadísticas.

El contenedor de café tambien es responsable de que su contenedor de café molido tenga disponibilidad para satisfacer los pedidos. Para ello
esta corriendo un loop que se fija si la cantidad de café molido disponible es mayor a una cierta cantidad definida. Cuando la misma es menor,
se va a bloquear el contenedor de café molido para que no pueda ser utilizado, y se va a moler algunos granos de café para que el contenedor de café molido 
permanezca lleno, o con la mayor cantidad posible.

Este loop finaliza cuando llega la señal de apagado porque no hay más pedidos, o cuando no hay más granos de café, ya que los mismos no se pueden reponer.

Cuando se alcanza un nivel definido de disponibilidad de granos, se imprime por pantalla una alerta.

### Contenedor de agua caliente (`HotWaterContainer`)
El contenedor de agua caliente es el encargado de llevar un registro de sus ingredientes disponibles y utilizados:
* `hot_water_container`: Es la cantidad de agua caliente disponible para el uso en un pedido.
* `used`:  Cantidad de agua caliente utilizada. Se va a usar para las estadísticas.

El contenedor de agua caliente tambien es responsable de que disponibilidad sea suficiente para satisfacer los pedidos. Para ello
esta corriendo un loop que se fija si la cantidad de agua caliente disponible es mayor a una cierta cantidad definida. Cuando la misma es menor,
se va a bloquear el contenedor para que no pueda ser utilizado, y se va a tomar agua de la red y calentarla, para luego rellenar el  contenedor de agua caliente.

Este loop finaliza cuando llega la señal de apagado porque no hay más pedidos. Nunca va a finalizar por quedarse sin agua ya que el mismo esta conectado a la red.

### Contenedor de leche (`MilkContainer`)
El contenedor de leche es el encargado de llevar un registro de sus ingredientes disponibles y utilizados:
* `milk_foam_container`: Es la cantidad de espuma de leche disponible para el uso en un pedido.
* `cold_milk_container`: Es la cantidad de leche fría disponible para ser utilizados para la reposición de espuma de leche.
* `milk_foam_used`:  Cantidad de espuma de leche utilizada. Se va a usar para las estadísticas.
* `cold_milk_used`: Cantidad de leche fría utilizada. Se va a usar para las estadísticas.

El contenedor de leche tambien es responsable de que su contenedor de espuma de leche tenga disponibilidad para satisfacer los pedidos. Para ello
esta corriendo un loop que se fija si la cantidad de espuma de leche disponible es mayor a una cierta cantidad definida. Cuando la misma es menor,
se va a bloquear el contenedor de espuma de leche para que no pueda ser utilizado, y se va a hacer espuma con la leche fría para que el contenedor de espuma de leche
permanezca lleno, o con la mayor cantidad posible.

Este loop finaliza cuando llega la señal de apagado porque no hay más pedidos, o cuando no hay más leche fría, ya que la misma no se puede reponer.

Cuando se alcanza un nivel definido de disponibilidad de leche fría, se imprime por pantalla una alerta.

### Contenedor de cacao (`CocoaContainer`)
El contenedor de cacao es el encargado de llevar un registro de sus ingredientes disponibles y utilizados:
* `cocoa_container`: Es la cantidad de cacao disponible para el uso en un pedido.
* `used`:  Cantidad de cacao utilizado. Se va a usar para las estadísticas.

El contenedor de cacao, a diferencia de los demás, no tiene forma de rellenarse cuando se está terminando su disponibilidad.

Cuando se alcanza un nivel definido de disponibilidad de cacao, se imprime por pantalla una alerta.

### Estadísticias
La cafetera corre un hilo aparte para la impresión de las estadísticas. Las mismas, cada un cierto valor definido de tiempo van a recolectar
la información que tienen los contenedores de ingredientes y la cantidad de pedidos que ya fueron realizados.

Una vez que llega la señal de apagado, se finaliza la tarea.

### Errores identificados
Hay algunos errores que no se me ocurrió cómo resolver y que los identifiqué haciendo tests.

* Corriendo el programa con el archivo [`src/tests/multiple_orders_cacao_overflow.txt`]. Esto hace un overflow de ingredientes con el cacao. Como necesita 110 de cacao
y nuestra capacidad es de 100, no puede realizar el pedido. Pero el resto de los ingredientes, si se chequearon antes del cacao van a ser servidos.
 Para evitarlo debería pedir el lock de cada uno de los contenedores y ver si lo puedo satisfacer, pero esto va a hacer que se pierda mucho tiempo en el chequeo y va a bloquear a muchos pedidos.
Tampoco puedo agregarlo a un unit test ya que las cantidades van a depender del orden en que se chequearon los ingredientes.

* Corriendo el programa con el archivo [`src/tests/multiple_orders.txt`] se llegan a completar todos los pedidos sin problemas. Pero si hay un problema si se quiere usar para correr un test unitario.
El test depende de en qué momento se terminan de preparar todos los pedidos. Como se utilizan 74 de café molido y el trigger para rellenar es cuando se utilizaron 70, seguramente llegue al trigger con el último hilo que agarra ese recurso.
Entonces es posible que mientras está esperando en el wait_while, tanto la condicion del trigger para rellenar como la del shutdown se validen al mismo tiempo. Si el shutdown es true para ese entonces, no rellena. No lo veo como un error del programa
ya que una vez que todos los pedidos fueron servidos no necesito rellenar los contenendores. Pero sí es un problema para hacer el test unitario porque no es consistente.
Voy a eliminar dicho test. Si quieren verlo está en el último commit.

* Estoy usando un tipo de error específico para algunos tests unitarios, pero igualmente el linter me dice que no estoy utilizando en ningún lado ese tipo de error. Por lo que
en la definición de  [`CustomError`] tuve que utilizar el decorador [`#![allow(dead_code)]`].
