package dev.gridio.dnp3.codegen.render

trait Indentation {

  def apply(f: => Iterator[String]): Iterator[String]

}