package dev.gridio.dnp3.codegen.render

class RenderString(value: String) {
  def iter : Iterator[String] = Iterator(value)
}

object Implicits {
  implicit def stringToRenderString(s : String) : RenderString = {
    new RenderString(s)
  }
}
