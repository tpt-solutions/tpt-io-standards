<?xml version="1.0" encoding="UTF-8"?>
<core:CityModel xmlns:core="http://www.opengis.net/citygml/2.0"
                xmlns:gml="http://www.opengis.net/gml"
                xmlns:bldg="http://www.opengis.net/citygml/building/2.0"
                xmlns:veg="http://www.opengis.net/citygml/vegetation/2.0"
                xmlns:wtr="http://www.opengis.net/citygml/waterbody/2.0"
                xmlns:tran="http://www.opengis.net/citygml/transportation/2.0">
  <core:boundedBy>
    <gml:Envelope srsName="urn:ogc:def:crs:EPSG::2193" srsDimension="3">
      <gml:lowerCorner>1748000 5426000 0</gml:lowerCorner>
      <gml:upperCorner>1748100 5426100 50</gml:upperCorner>
    </gml:Envelope>
  </core:boundedBy>
  <core:cityObjectMember>
    <bldg:Building gml:id="bldg-1">
      <bldg:function>1000</bldg:function>
      <bldg:yearOfConstruction>1998</bldg:yearOfConstruction>
      <bldg:lod1Solid>
        <gml:Solid>
          <gml:exterior>
            <gml:CompositeSurface>
              <gml:surfaceMember>
                <gml:Polygon>
                  <gml:exterior>
                    <gml:LinearRing>
                      <gml:posList>0 0 0 10 0 0 10 10 0 0 10 0 0 0 0</gml:posList>
                    </gml:LinearRing>
                  </gml:exterior>
                </gml:Polygon>
              </gml:surfaceMember>
            </gml:CompositeSurface>
          </gml:exterior>
        </gml:Solid>
      </bldg:lod1Solid>
    </bldg:Building>
  </core:cityObjectMember>
  <core:cityObjectMember>
    <veg:PlantCover gml:id="veg-1">
      <veg:lod1MultiSurface>
        <gml:MultiSurface>
          <gml:surfaceMember>
            <gml:Polygon>
              <gml:exterior>
                <gml:LinearRing>
                  <gml:posList>20 20 0 30 20 0 30 30 0 20 30 0 20 20 0</gml:posList>
                </gml:LinearRing>
              </gml:exterior>
            </gml:Polygon>
          </gml:surfaceMember>
        </gml:MultiSurface>
      </veg:lod1MultiSurface>
    </veg:PlantCover>
  </core:cityObjectMember>
  <core:cityObjectMember>
    <wtr:WaterBody gml:id="wtr-1">
      <wtr:lod0MultiSurface>
        <gml:MultiSurface>
          <gml:surfaceMember>
            <gml:Polygon>
              <gml:exterior>
                <gml:LinearRing>
                  <gml:posList>40 40 0 60 40 0 60 60 0 40 60 0 40 40 0</gml:posList>
                </gml:LinearRing>
              </gml:exterior>
            </gml:Polygon>
          </gml:surfaceMember>
        </gml:MultiSurface>
      </wtr:lod0MultiSurface>
    </wtr:WaterBody>
  </core:cityObjectMember>
  <core:cityObjectMember>
    <tran:Road gml:id="road-1">
      <tran:lod1MultiSurface>
        <gml:MultiSurface>
          <gml:surfaceMember>
            <gml:Polygon>
              <gml:exterior>
                <gml:LinearRing>
                  <gml:posList>0 50 0 100 50 0 100 52 0 0 52 0 0 50 0</gml:posList>
                </gml:LinearRing>
              </gml:exterior>
            </gml:Polygon>
          </gml:surfaceMember>
        </gml:MultiSurface>
      </tran:lod1MultiSurface>
    </tran:Road>
  </core:cityObjectMember>
</core:CityModel>
