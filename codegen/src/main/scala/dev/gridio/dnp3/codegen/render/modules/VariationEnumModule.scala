package dev.gridio.dnp3.codegen.render.modules

import dev.gridio.dnp3.codegen.model.{AnyVariation, FixedSize, ObjectGroup, SizedByVariation, Variation}
import dev.gridio.dnp3.codegen.render._

object VariationEnumModule extends Module {

  override def lines(implicit indent: Indentation) : Iterator[String] = {
    enumDefinition ++ space ++ enumImpl
  }

  private def enumDefinition(implicit indent: Indentation) : Iterator[String] = {

    def getVariationDefinition(v: Variation) : String = v match {
      case v : SizedByVariation => s"${v.parent.name}(u8)"
      case _ =>  s"${v.name}"
    }

    "#[derive(Copy, Clone, Debug, PartialEq)]".eol ++
      bracket("pub enum Variation") {
        ObjectGroup.allVariations.iterator.flatMap(v => s"${getVariationDefinition(v)},".eol)
      }
  }

  private def enumImpl(implicit indent: Indentation) : Iterator[String] = {

    def isSizedByVariation(g: ObjectGroup) : Boolean = {
      g.variations match {
        case List(_ : SizedByVariation) => true
        case _ => false
      }
    }

    def matchVariation(g : ObjectGroup): Iterator[String] = {

      if (isSizedByVariation(g)) {
        s"${g.group} => Some(Variation::${g.name}(var)),".eol
      } else {
        bracketComma(s"${g.group} => match var") {
          g.variations.iterator.flatMap { v =>
            s"${v.variation} => Some(Variation::${v.name}),".eol
          } ++ "_ => None,".eol
        }
      }


    }

    bracket("impl Variation") {
      bracket("pub fn lookup(group: u8, var: u8) -> Option<Variation>") {
        bracket("match group") {
          ObjectGroup.all.iterator.flatMap(matchVariation) ++ "_ => None,".eol
        }
      }
    }

  }




}
