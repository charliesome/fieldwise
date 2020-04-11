pub trait Fieldwise {
    type Root;
    type Fields;

    fn root() -> Self::Root;
    fn fieldwise() -> Self::Fields;
}

pub trait Path {
    type Root: Path;
    type Item;

    fn get<'a>(&self, root: &'a <Self::Root as Path>::Item) -> Option<&'a Self::Item>;
    fn get_mut<'a>(&self, root: &'a mut <Self::Root as Path>::Item) -> Option<&'a mut Self::Item>;
}

pub struct Compose<A, B>(pub A, pub B);

impl<A: Path<Item = Ap::Item>, B: Path<Root = Ap>, Ap: Path + 'static> Path for Compose<A, B> {
    type Root = A::Root;
    type Item = B::Item;

    fn get<'a>(&self, root: &'a <Self::Root as Path>::Item) -> Option<&'a Self::Item> {
        self.1.get(self.0.get(root)?)
    }

    fn get_mut<'a>(&self, root: &'a mut <Self::Root as Path>::Item) -> Option<&'a mut Self::Item> {
        self.1.get_mut(self.0.get_mut(root)?)
    }
}
