use crate::tag_processor::ParsingNamespace;

pub fn qualified_attribute_name(attribute_name: &[u8], ns: &ParsingNamespace) -> Box<[u8]> {
    let lower_name = attribute_name.to_ascii_lowercase();
    let d = String::from_utf8_lossy(attribute_name);
    dbg!(d, ns);

    if ns != &ParsingNamespace::Html {
        let transformed = match lower_name.as_slice() {
            b"xlink:actuate" => Some(b"xlink actuate".as_slice()),
            b"xlink:arcrole" => Some(b"xlink arcrole".as_slice()),
            b"xlink:href" => Some(b"xlink href".as_slice()),
            b"xlink:role" => Some(b"xlink role".as_slice()),
            b"xlink:show" => Some(b"xlink show".as_slice()),
            b"xlink:title" => Some(b"xlink title".as_slice()),
            b"xlink:type" => Some(b"xlink type".as_slice()),
            b"xml:lang" => Some(b"xml lang".as_slice()),
            b"xml:space" => Some(b"xml space".as_slice()),
            b"xmlns" => Some(b"xmlns".as_slice()),
            b"xmlns:xlink" => Some(b"xmlns xlink".as_slice()),
            _ => None,
        };
        if let Some(transformed) = transformed {
            return transformed.into();
        }
    }

    match ns {
        ParsingNamespace::MathML if lower_name == b"definitionurl" => {
            b"definitionURL".as_slice().into()
        }
        ParsingNamespace::Svg => match lower_name.as_slice() {
            b"attributename" => b"attributeName".as_slice(),
            b"attributetype" => b"attributeType",
            b"basefrequency" => b"baseFrequency",
            b"baseprofile" => b"baseProfile",
            b"calcmode" => b"calcMode",
            b"clippathunits" => b"clipPathUnits",
            b"diffuseconstant" => b"diffuseConstant",
            b"edgemode" => b"edgeMode",
            b"filterunits" => b"filterUnits",
            b"glyphref" => b"glyphRef",
            b"gradienttransform" => b"gradientTransform",
            b"gradientunits" => b"gradientUnits",
            b"kernelmatrix" => b"kernelMatrix",
            b"kernelunitlength" => b"kernelUnitLength",
            b"keypoints" => b"keyPoints",
            b"keysplines" => b"keySplines",
            b"keytimes" => b"keyTimes",
            b"lengthadjust" => b"lengthAdjust",
            b"limitingconeangle" => b"limitingConeAngle",
            b"markerheight" => b"markerHeight",
            b"markerunits" => b"markerUnits",
            b"markerwidth" => b"markerWidth",
            b"maskcontentunits" => b"maskContentUnits",
            b"maskunits" => b"maskUnits",
            b"numoctaves" => b"numOctaves",
            b"pathlength" => b"pathLength",
            b"patterncontentunits" => b"patternContentUnits",
            b"patterntransform" => b"patternTransform",
            b"patternunits" => b"patternUnits",
            b"pointsatx" => b"pointsAtX",
            b"pointsaty" => b"pointsAtY",
            b"pointsatz" => b"pointsAtZ",
            b"preservealpha" => b"preserveAlpha",
            b"preserveaspectratio" => b"preserveAspectRatio",
            b"primitiveunits" => b"primitiveUnits",
            b"refx" => b"refX",
            b"refy" => b"refY",
            b"repeatcount" => b"repeatCount",
            b"repeatdur" => b"repeatDur",
            b"requiredextensions" => b"requiredExtensions",
            b"requiredfeatures" => b"requiredFeatures",
            b"specularconstant" => b"specularConstant",
            b"specularexponent" => b"specularExponent",
            b"spreadmethod" => b"spreadMethod",
            b"startoffset" => b"startOffset",
            b"stddeviation" => b"stdDeviation",
            b"stitchtiles" => b"stitchTiles",
            b"surfacescale" => b"surfaceScale",
            b"systemlanguage" => b"systemLanguage",
            b"tablevalues" => b"tableValues",
            b"targetx" => b"targetX",
            b"targety" => b"targetY",
            b"textlength" => b"textLength",
            b"viewbox" => b"viewBox",
            b"viewtarget" => b"viewTarget",
            b"xchannelselector" => b"xChannelSelector",
            b"ychannelselector" => b"yChannelSelector",
            b"zoomandpan" => b"zoomAndPan",
            _ => &lower_name,
        }
        .into(),
        _ => lower_name.into(),
    }
}
