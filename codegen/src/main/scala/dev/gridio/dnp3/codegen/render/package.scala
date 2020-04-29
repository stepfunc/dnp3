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

  def quoted(s: String): String = s""""${s}""""

  def spaced(groups: Iterator[Iterator[String]]): Iterator[String] = {
    groups.foldLeft(Iterator.empty[String])((x, sum) => sum ++ space ++ x)
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

  def parenSemi(s: String)(inner: => Iterator[String])(implicit indent: Indentation): Iterator[String] = {
    (s + "(").eol ++ indent {
      inner
    } ++ ");".eol
  }
}
