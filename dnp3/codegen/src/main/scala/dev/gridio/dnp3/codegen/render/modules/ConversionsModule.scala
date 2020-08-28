package dev.gridio.dnp3.codegen.render.modules

import dev.gridio.dnp3.codegen.model._
import dev.gridio.dnp3.codegen.render._

object ConversionsModule extends Module {

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

  def flagsConversion(fs: FixedSize) : String = {
    def hasFlags : Boolean = {
      fs.fields.exists(f => f.attr.contains(FieldAttribute.Flags))
    }
    if(hasFlags) {
      "Flags::new(v.flags)"
    } else {
      "Flags::ONLINE"
    }
  }

  private def binaryConversions(implicit indentation: Indentation) : Iterator[String] = {

    def binaryVariations : List[FixedSize] = {
      def isNonCTO(fs: FixedSize) : Boolean = {
        val isType = fs.parent.groupType == GroupType.StaticBinary || fs.parent.groupType == GroupType.BinaryEvent
        isType && !fs.hasRelativeTime
      }
      fixedSize(isNonCTO)
    }

    def binaryOutputStatusVariations : List[FixedSize] = {
      def isNonCTO(fs: FixedSize) : Boolean = {
        val isType = fs.parent.groupType == GroupType.StaticBinaryOutputStatus || fs.parent.groupType == GroupType.BinaryOutputEvent
        isType && !fs.hasRelativeTime
      }
      fixedSize(isNonCTO)
    }

    def single(name: String)(fs: FixedSize) : Iterator[String] = {
      def conversion : Iterator[String] = {
        "let flags = Flags::new(v.flags);".eol ++
          bracket(s"${name}") {
            "value : flags.state(),".eol ++
              "flags,".eol ++
              s"time : ${timeConversion(fs)},".eol
          }
      }

      bracket(s"impl From<${fs.name}> for ${name}") {
        bracket(s"fn from(v: ${fs.name}) -> Self") {
          conversion
        }
      }
    }

    spaced(binaryVariations.map(single("Binary")).iterator) ++
    space ++
    spaced(binaryOutputStatusVariations.map(single("BinaryOutputStatus")).iterator)
  }

  private def doubleBitBinaryConversions(implicit indentation: Indentation) : Iterator[String] = {

    def variations : List[FixedSize] = {
      def isNonCTO(fs: FixedSize) : Boolean = {
        val isType = fs.parent.groupType == GroupType.StaticDoubleBinary || fs.parent.groupType == GroupType.DoubleBinaryEvent
        isType && !fs.hasRelativeTime
      }
      fixedSize(isNonCTO)
    }

    def single(fs: FixedSize) : Iterator[String] = {
      def conversion : Iterator[String] = {
        "let flags = Flags::new(v.flags);".eol ++
          bracket("DoubleBitBinary") {
            "value : flags.double_bit_state(),".eol ++
              "flags,".eol ++
              s"time : ${timeConversion(fs)},".eol
          }
      }

      bracket(s"impl From<${fs.name}> for DoubleBitBinary") {
        bracket(s"fn from(v: ${fs.name}) -> Self") {
          conversion
        }
      }
    }

    spaced(variations.map(single).iterator)
  }

  private def counterConversions(implicit indentation: Indentation) : Iterator[String] = {

    def counterVariations : List[FixedSize] = {
      def isType(fs: FixedSize) : Boolean = {
        fs.parent.groupType == GroupType.StaticCounter || fs.parent.groupType == GroupType.CounterEvent
      }
      fixedSize(isType)
    }

    def frozenCounterVariations : List[FixedSize] = {
      def isType(fs: FixedSize) : Boolean = {
        fs.parent.groupType == GroupType.StaticFrozenCounter || fs.parent.groupType == GroupType.FrozenCounterEvent
      }
      fixedSize(isType)
    }

    def single(name: String)(fs: FixedSize) : Iterator[String] = {
      def conversion : Iterator[String] = {
        def cast : String = {
          val field = fs.fields.find(f => f.attr.contains(FieldAttribute.Value)).get
          field.typ match {
            case UInt32Field => ""
            case _ => " as u32"
          }
        }
        bracket(name) {
          s"value : v.value${cast},".eol ++
            s"flags: ${flagsConversion(fs)},".eol ++
            s"time : ${timeConversion(fs)},".eol
        }
      }

      bracket(s"impl From<${fs.name}> for ${name}") {
        bracket(s"fn from(v: ${fs.name}) -> Self") {
          conversion
        }
      }
    }

    spaced(counterVariations.map(single("Counter")).iterator) ++
    space ++
    spaced(frozenCounterVariations.map(single("FrozenCounter")).iterator)
  }

  private def analogConversions(implicit indentation: Indentation) : Iterator[String] = {

    def analogVariations : List[FixedSize] = {
      def isType(fs: FixedSize) : Boolean = {
        fs.parent.groupType == GroupType.StaticAnalog || fs.parent.groupType == GroupType.AnalogEvent
      }
      fixedSize(isType)
    }

    def analogOutputStatusVariations : List[FixedSize] = {
      def isType(fs: FixedSize) : Boolean = {
        fs.parent.groupType == GroupType.StaticAnalogOutputStatus || fs.parent.groupType == GroupType.AnalogOutputEvent
      }
      fixedSize(isType)
    }

    def single(name: String)(fs: FixedSize) : Iterator[String] = {
      def conversion : Iterator[String] = {
        def cast : String = {
          val field = fs.fields.find(f => f.attr.contains(FieldAttribute.Value)).get
          field.typ match {
            case Float64Field => ""
            case _ => " as f64"
          }
        }
        bracket(name) {
          s"value : v.value${cast},".eol ++
          s"flags: ${flagsConversion(fs)},".eol ++
          s"time : ${timeConversion(fs)},".eol
        }
      }

      bracket(s"impl From<${fs.name}> for ${name}") {
        bracket(s"fn from(v: ${fs.name}) -> Self") {
          conversion
        }
      }
    }

    spaced(analogVariations.map(single("Analog")).iterator) ++
    space ++
    spaced(analogOutputStatusVariations.map(single("AnalogOutputStatus")).iterator)
  }

  override def lines(implicit indentation: Indentation): Iterator[String] = {
    "use crate::app::measurement::*;".eol ++
    "use crate::app::flags::*;".eol ++
    "use crate::app::variations::*;".eol ++
    space ++
    binaryConversions ++
    space ++
    doubleBitBinaryConversions ++
    space ++
    counterConversions ++
    space ++
    analogConversions
  }


}
