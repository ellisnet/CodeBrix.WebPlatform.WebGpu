using CodeBrix.WebPlatform.Core.Mapping;

//What this backend declares about itself for the generated frontend code: the value every
//state-changing request must carry in X-Requested-With. The platform's backend library reads it
//back from here, and the generated client code of every frontend application sends it.
[assembly: WireRequestMarker("GraphicsDemo")]
