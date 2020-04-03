package dev.gridio.dnp3.codegen.render.modules

import dev.gridio.dnp3.codegen.model._
import dev.gridio.dnp3.codegen.render._

object ConversionsModule extends Module {

  def isBinary(fs: FixedSize) : Boolean = {
    fs.parent.groupType == GroupType.StaticBinary || fs.parent.groupType == GroupType.BinaryEvent
  }

  private def fixedSize(select: FixedSize => Boolean) : List[FixedSize] = {
    ObjectGroup.allVariations.flatMap { v=>
      v match {
        case fs : FixedSize if select(fs) => Some(fs)
        case _ => None
      }
    }
  }

  def timeConversion(fs: FixedSize) : String = {
    def hasTime48 : Boolean = {
      fs.fields.exists(f => f.attr.contains(FieldAttribute.Timestamp48))
    }
    if(hasTime48) {
      "Time::Synchronized(v.time)"
    } else {
      "Time::Invalid"
    }
  }
  
  private def binaryVariations : List[FixedSize] = {
    def isNonCTOBinary(fs: FixedSize) : Boolean = {
      val isBinary = fs.parent.groupType == GroupType.StaticBinary || fs.parent.groupType == GroupType.BinaryEvent
      isBinary && !fs.hasRelativeTime
    }
    fixedSize(isNonCTOBinary)
  }

  private def binaryConversion(fs: FixedSize)(implicit indentation: Indentation) : Iterator[String] = {

    def conversion : Iterator[String] = {
      "let flags = Flags::new(v.flags);".eol ++
      bracket("Binary") {
        "value : flags.state(),".eol ++
        "flags,".eol ++
        s"time : ${timeConversion(fs)},".eol
      }
    }


    bracket(s"impl std::convert::From<${fs.name}> for Binary") {
      bracket(s"fn from(v: ${fs.name}) -> Self") {
        conversion
      }
    }
  }

  override def lines(implicit indentation: Indentation): Iterator[String] = {
    "use crate::app::meas::*;".eol ++
    "use crate::app::gen::variations::fixed::*;".eol ++
    "use crate::app::flags::Flags;".eol ++
    space ++
    spaced(binaryVariations.map(binaryConversion).iterator)
  }


}
