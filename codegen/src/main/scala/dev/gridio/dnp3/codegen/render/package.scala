package dev.gridio.dnp3.codegen

package object render {
  class RenderString(value: String) {
    def eol : Iterator[String] = Iterator(value)
  }



  implicit def stringToRenderString(s : String) : RenderString = {
    new RenderString(s)
  }

  def space: Iterator[String] = Iterator("")

  def commented(s: String): String = s"/// ${s}"

  def spaced(groups: Iterator[Iterator[String]]): Iterator[String] = {

    var sum : Iterator[String] = Iterator.empty[String]

    groups.foreach { x : Iterator[String] =>
      sum = sum ++ (if(groups.hasNext) x ++ space else x)
    }
    sum
  }

  def bracket(s: String)(inner: => Iterator[String])(implicit indent: Indentation): Iterator[String] = {
    (s + " {").eol ++ indent {
      inner
    } ++ "}".eol
  }

  def bracketComma(s: String)(inner: => Iterator[String])(implicit indent: Indentation): Iterator[String] = {
    (s + " {").eol ++ indent {
      inner
    } ++ "},".eol
  }

  def paren(s: String)(inner: => Iterator[String])(implicit indent: Indentation): Iterator[String] = {
    (s + "(").eol ++ indent {
      inner
    } ++ ")".eol
  }
}
