use super::{Face, Shell, Solid};
use crate::primitives::{IntoShape, Shape, ShapeType};
use cxx::UniquePtr;
use glam::{dvec3, DVec3};
use opencascade_sys::ffi;

pub struct Compound {
    pub(crate) inner: UniquePtr<ffi::TopoDS_Compound>,
}

impl Clone for Compound {
    fn clone(&self) -> Self {
        let source = ffi::cast_compound_to_shape(&self.inner);
        let mut copier = ffi::BRepBuilderAPI_Copy_ctor(source, true, false);
        let target = copier.pin_mut().Shape();
        let compound = ffi::TopoDS_cast_to_compound(target);
        Compound::from_compound(compound)
    }
}

impl AsRef<Compound> for Compound {
    fn as_ref(&self) -> &Compound {
        self
    }
}

impl TryFrom<&Shape> for Compound {
    type Error = cxx::Exception;

    fn try_from(value: &Shape) -> Result<Self, Self::Error> {
        ffi::try_cast_TopoDS_to_compound(&value.inner).map(Self::from_compound)
    }
}

impl Compound {
    pub(crate) fn from_compound(compound: &ffi::TopoDS_Compound) -> Self {
        let inner = ffi::TopoDS_Compound_to_owned(compound);

        Self { inner }
    }

    #[must_use]
    pub fn clean(&self) -> Shape {
        let shape = ffi::cast_compound_to_shape(&self.inner);

        Shape::from_shape(shape).clean()
    }

    pub fn from_shapes<T: AsRef<Shape>>(shapes: impl IntoIterator<Item = T>) -> Self {
        let mut compound = ffi::TopoDS_Compound_ctor();
        let builder = ffi::BRep_Builder_ctor();
        let builder = ffi::BRep_Builder_upcast_to_topods_builder(&builder);
        builder.MakeCompound(compound.pin_mut());
        let mut compound_shape = ffi::TopoDS_Compound_as_shape(compound);

        for shape in shapes.into_iter() {
            builder.Add(compound_shape.pin_mut(), &shape.as_ref().inner);
        }

        let compound = ffi::TopoDS_cast_to_compound(&compound_shape);
        Self::from_compound(compound)
    }

    pub fn center_of_mass(&self) -> DVec3 {
        let mut props = ffi::GProp_GProps_ctor();

        let inner_shape = ffi::cast_compound_to_shape(&self.inner);
        ffi::BRepGProp_SurfaceProperties(inner_shape, props.pin_mut());

        let center = ffi::GProp_GProps_CentreOfMass(&props);

        dvec3(center.X(), center.Y(), center.Z())
    }

    pub fn volume<'a>(
        fs: impl IntoIterator<Item = &'a Face>,
        ls: impl IntoIterator<Item = &'a Shell>,
        ds: impl IntoIterator<Item = &'a Solid>,
    ) -> Result<Compound, cxx::Exception> {
        // create volume maker
        let mut maker = ffi::BOPAlgo_MakerVolume_ctor();

        // set arguments to make solid from
        let mut arguments = ffi::new_list_of_shape();
        for shape in fs
            .into_iter()
            .map(|face| face.into_shape())
            .chain(ls.into_iter().map(|shell| shell.into_shape()))
            .chain(ds.into_iter().map(|solid| solid.into_shape()))
        {
            ffi::shape_list_append_shape(arguments.pin_mut(), &shape.as_ref().inner);
        }
        maker.pin_mut().SetArguments(&arguments);

        // perform the opearation
        maker.pin_mut().Perform(&ffi::Message_ProgressRange_ctor());

        let result_shape = ffi::BOPAlgo_MakerVolume_Shape(&maker);
        Ok(if let ShapeType::Compound = ShapeType::from(result_shape.ShapeType()) {
            ffi::try_cast_TopoDS_to_compound(result_shape).map(Compound::from_compound)?
        } else {
            Compound::from_shapes([Shape::from_shape(result_shape)])
        })
    }
}
