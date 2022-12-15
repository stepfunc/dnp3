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
      "Some(Time::Synchronized(v.time))"
    } else {
      "None"
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
      def variationToMeas : Iterator[String] = {
        "let flags = Flags::new(v.flags);".eol ++
          bracket(s"${name}") {
            "value : flags.state(),".eol ++
              "flags,".eol ++
              s"time : ${timeConversion(fs)},".eol
          }
      }

      def measToVariation : Iterator[String] = {
          def fieldGetter(field: FixedSizeField): String = {
            field.typ match {
              case TimestampField => "self.time.into()"
              case UInt8Field if field.isFlags => "self.get_wire_flags()"
            }
          }

          bracket(s"${fs.name}") {
            fs.fields.map(f => s"${f.name}: ${fieldGetter(f)},").iterator
          }
      }

      bracket(s"impl From<${fs.name}> for ${name}") {
        bracket(s"fn from(v: ${fs.name}) -> Self") {
          variationToMeas
        }
      } ++ space ++
        bracket(s"impl ToVariation<${fs.name}> for ${name}") {
          bracket(s"fn to_variation(&self) -> ${fs.name}") {
            measToVariation
          }
        }
    }

    spaced(binaryVariations.map(single("BinaryInput")).iterator) ++
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
      def variationToMeas : Iterator[String] = {
        "let flags = Flags::new(v.flags);".eol ++
          bracket("DoubleBitBinaryInput") {
            "value : flags.double_bit_state(),".eol ++
              "flags,".eol ++
              s"time : ${timeConversion(fs)},".eol
          }
      }

      def measToVariation : Iterator[String] = {
        def fieldGetter(field: FixedSizeField): String = {
          field.typ match {
            case TimestampField => "self.time.into()"
            case UInt8Field if field.isFlags => "self.get_wire_flags()"
          }
        }

        bracket(s"${fs.name}") {
          fs.fields.map(f => s"${f.name}: ${fieldGetter(f)},").iterator
        }
      }

      bracket(s"impl From<${fs.name}> for DoubleBitBinaryInput") {
        bracket(s"fn from(v: ${fs.name}) -> Self") {
          variationToMeas
        }
      } ++ space ++
        bracket(s"impl ToVariation<${fs.name}> for DoubleBitBinaryInput") {
          bracket(s"fn to_variation(&self) -> ${fs.name}") {
            measToVariation
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
      def variationToMeas : Iterator[String] = {
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

      def measToVariation : Iterator[String] = {
        def fieldGetter(field: FixedSizeField): String = {
          field.typ match {
            case TimestampField => "self.time.into()"
            case UInt16Field if field.isValue => "self.value as u16"
            case UInt32Field if field.isValue => "self.value"
            case UInt8Field if field.isFlags => "self.flags.value"
          }
        }

        bracket(s"${fs.name}") {
          fs.fields.map(f => s"${f.name}: ${fieldGetter(f)},").iterator
        }
      }



      bracket(s"impl From<${fs.name}> for ${name}") {
        bracket(s"fn from(v: ${fs.name}) -> Self") {
          variationToMeas
        }
      } ++ space ++
        bracket(s"impl ToVariation<${fs.name}> for ${name}") {
          bracket(s"fn to_variation(&self) -> ${fs.name}") {
            measToVariation
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

    def frozenAnalogVariations: List[FixedSize] = {
      def isType(fs: FixedSize): Boolean = {
        fs.parent.groupType == GroupType.StaticFrozenAnalog || fs.parent.groupType == GroupType.FrozenAnalogEvent
      }

      fixedSize(isType)
    }

    def single(name: String)(fs: FixedSize) : Iterator[String] = {
      def variationToMeas : Iterator[String] = {
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

      def hasDoubleValue: Boolean = {
        fs.fields.exists(f => {
          f.isValue && {
            f.typ match {
              case Float64Field => true
              case _ => false
            }
          }
        })
      }

      def measToVariation : Iterator[String] = {
        def fieldGetter(field: FixedSizeField): String = {
          field.typ match {
            case TimestampField => "self.time.into()"
            case UInt8Field if field.isFlags => "self.flags.value"
            case EnumFieldType(_) => s"self.${field.name}"
            case _ if field.isValue => "self.value"
          }
        }

        bracket(s"${fs.name}") {
          fs.fields.map(f => s"${f.name}: ${fieldGetter(f)},").iterator
        }
      }

      def measToVariationWithCast : Iterator[String] = {
        def getFlagsAndValue: String = {
           fs.fields.find(_.isValue).get.typ match {
             case S16Field => "self.to_i16()"
             case S32Field => "self.to_i32()"
             case Float32Field => "self.to_f32()"
           }
        }

        def fieldGetter(field: FixedSizeField): String = {
          field.typ match {
            case TimestampField => "self.time.into()"
            case UInt8Field if field.isFlags => "_wire_flags.value"
            case EnumFieldType(_) => s"self.${field.name}"
            case _ if field.isValue => "_wire_value"
          }
        }

        s"let (_wire_flags, _wire_value) = ${getFlagsAndValue};".eol ++
        bracket(s"${fs.name}") {
          fs.fields.map(f => s"${f.name}: ${fieldGetter(f)},").iterator
        }
      }

      def toVariation: Iterator[String] = {
        if(fs.parent.groupType == GroupType.AnalogOutputCommandEvent) {
          Iterator.empty
        } else {
          space ++ bracket(s"impl ToVariation<${fs.name}> for ${name}") {
            bracket(s"fn to_variation(&self) -> ${fs.name}") {
              if(hasDoubleValue) measToVariation else measToVariationWithCast
            }
          }
        }
      }

      bracket(s"impl From<${fs.name}> for ${name}") {
        bracket(s"fn from(v: ${fs.name}) -> Self") {
          variationToMeas
        }
      } ++ toVariation

    }

    spaced(analogVariations.map(single("AnalogInput")).iterator) ++
    space ++
    spaced(analogOutputStatusVariations.map(single("AnalogOutputStatus")).iterator) ++
    space ++
    spaced(frozenAnalogVariations.map(single("FrozenAnalogInput")).iterator)
  }

  override def lines(implicit indentation: Indentation): Iterator[String] = {
    "use crate::app::measurement::*;".eol ++
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
