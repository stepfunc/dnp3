package dev.gridio.dnp3.codegen

package object render {
  class RenderString(value: String) {
    def eol : Iterator[String] = Iterator(value)
  }


  implicit def stringToRenderString(s : String) : RenderString = {
    new RenderString(s)
  }

  def space: Iterator[String] = Iterator("")

  def spaced(i: Iterator[Iterator[String]]): Iterator[String] = {

    def map(iter: Iterator[String]): Iterator[String] = if(iter.hasNext) iter ++ space else iter

    i.foldLeft(Iterator.apply[String]())((sum, i) => sum ++ map(i))
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
