package dev.gridio.dnp3.codegen.render.modules

import dev.gridio.dnp3.codegen.model.groups.SpecificAttribute
import dev.gridio.dnp3.codegen.model.{AnyVariation, FixedSize, GroupType, ObjectGroup, SizedByVariation, Variation}
import dev.gridio.dnp3.codegen.render._

object VariationEnumModule extends Module {

  override def lines(implicit indent: Indentation) : Iterator[String] = {
    enumDefinition ++ space ++ enumImpl
  }

  private def enumDefinition(implicit indent: Indentation) : Iterator[String] = {

    def getVariationDefinition(v: Variation) : String = v match {
      case _ : SizedByVariation => s"${v.parent.name}(u8)"
      case SpecificAttribute => s"${v.parent.name}(u8)"
      case _ =>  s"${v.name}"
    }

      "/// All variations supported by the library".eol ++
      "#[derive(Copy, Clone, Debug, PartialEq, Eq)]".eol ++
      bracket("pub enum Variation") {
        ObjectGroup.allVariations.iterator.flatMap {
          v => {
            s"/// ${v.fullDesc}".eol ++
            s"${getVariationDefinition(v)},".eol
          }
        }
      }
  }

  private def enumImpl(implicit indent: Indentation) : Iterator[String] = {

    def isSizedByVariation(g: ObjectGroup) : Boolean = {
      g.variations match {
        case List(_ : SizedByVariation) => true
        case _ => false
      }
    }

    def lookupFn : Iterator[String] = {
      def matchVariation(g : ObjectGroup): Iterator[String] = {
        g.groupType match {
          case GroupType.StaticOctetString | GroupType.OctetStringEvent =>
            s"${g.group} => Some(Variation::${g.name}(var)),".eol
          case GroupType.DeviceAttributes =>
            bracketComma(s"${g.group} => match var") {
              "0 => None,".eol ++ // not allowed
              "254 => Some(Variation::Group0Var254),".eol ++
              "_ => Some(Variation::Group0(var)),".eol
            }
          case _ =>
            bracketComma(s"${g.group} => match var") {
              g.variations.iterator.flatMap { v =>
                s"${v.variation} => Some(Variation::${v.name}),".eol
              } ++ "_ => None,".eol
            }
        }
      }

      bracket("pub(crate) fn lookup(group: u8, var: u8) -> Option<Variation>") {
        bracket("match group") {
          ObjectGroup.all.iterator.flatMap(matchVariation) ++ "_ => None,".eol
        }
      }
    }

    def getGroupVarFn : Iterator[String] = {
      def matcher(v : Variation): Iterator[String] = {
        v match {
          case _ : SizedByVariation => {
            s"Variation::${v.parent.name}(x) => (${v.parent.group}, x),".eol
          }
          case SpecificAttribute => {
            s"Variation::${v.parent.name}(x) => (${v.parent.group}, x),".eol
          }
          case _ => {
            s"Variation::${v.name} => (${v.group}, ${v.variation}),".eol
          }
        }
      }

      bracket("pub(crate) fn to_group_and_var(self) -> (u8, u8)") {
        bracket("match self") {
          ObjectGroup.allVariations.iterator.flatMap(matcher)
        }
      }
    }

    def descriptionFn : Iterator[String] = {
      def matcher(v : Variation): Iterator[String] = {
        v match {
          case _ : SizedByVariation => {
            s"Variation::${v.parent.name}(_) => ${quoted(v.fullDesc)},".eol
          }
          case SpecificAttribute => {
            s"Variation::${v.parent.name}(_) => ${quoted(v.fullDesc)},".eol
          }
          case _ => {
            s"Variation::${v.name} => ${quoted(v.fullDesc)},".eol
          }
        }
      }

      bracket("pub(crate) fn description(self) -> &'static str") {
        bracket("match self") {
          ObjectGroup.allVariations.iterator.flatMap(matcher)
        }
      }
    }



    bracket("impl Variation") {
      lookupFn ++ space ++ getGroupVarFn ++ space ++ descriptionFn
    }

  }




}
