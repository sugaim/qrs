use anyhow::ensure;

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub(super) struct Knots<G, V> {
    gs: Vec<G>,
    vs: Vec<V>,
}

impl<G: PartialOrd, V> Knots<G, V> {
    // construction
    pub fn new(gs: Vec<G>, vs: Vec<V>) -> Result<Self, anyhow::Error> {
        ensure!(
            gs.len() == vs.len(),
            "Lengths of grids and values must be the same"
        );
        ensure!(2 <= gs.len(), "At least two knots are required");
        ensure!(
            gs.windows(2).all(|w| w[0] < w[1]),
            "Grids must be sorted in ascending order"
        );
        Ok(Self { gs, vs })
    }

    // accessors
    pub fn grids(&self) -> &[G] {
        &self.gs
    }
    pub fn values(&self) -> &[V] {
        &self.vs
    }
    pub fn destruct(self) -> (Vec<G>, Vec<V>) {
        (self.gs, self.vs)
    }
    pub fn force_get(&self, idx: usize) -> (&G, &V) {
        (&self.gs[idx], &self.vs[idx])
    }

    /// Get the index of interval (left: close, right: open) that contains the given point.
    ///
    /// ```txt
    /// -x--[0]-----[1]-----[2]---- => 0
    /// ----[0]=x---[1]-----[2]---- => 0
    /// ----[0]--x--[1]-----[2]---- => 0
    /// ----[0]-----[1]=x---[2]---- => 1
    /// ----[0]-----[1]--x--[2]---- => 1
    /// ----[0]-----[1]-----[2]=x-- => 1
    /// ----[0]-----[1]-----[2]--x- => 1
    /// ```
    ///
    /// # Examples
    /// ```ignore
    /// use qcore::interp1d::lerp::Knots;
    ///
    /// let knots = Knots::new(vec![0, 10, 20], vec![0, 1, 2]).unwrap();
    ///
    /// assert_eq!(knots.interval_index_of(&-1), 0);
    /// assert_eq!(knots.interval_index_of(&0), 0);
    /// assert_eq!(knots.interval_index_of(&5), 0);
    /// assert_eq!(knots.interval_index_of(&10), 1);
    /// assert_eq!(knots.interval_index_of(&15), 1);
    /// assert_eq!(knots.interval_index_of(&20), 1);
    /// assert_eq!(knots.interval_index_of(&25), 1);
    /// ```
    ///
    pub fn interval_index_of(&self, x: &G) -> usize {
        assert!(2 <= self.gs.len(), "ctor must guarantee at least two knots");
        let idx = self.gs[0..self.gs.len() - 1].partition_point(|g| g <= x);
        if idx == 0 {
            0
        } else {
            idx - 1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interval_index_of() {
        let knots = Knots::new(vec![0, 10, 20], vec![0, 1, 2]).unwrap();
        assert_eq!(knots.interval_index_of(&-1), 0);
        assert_eq!(knots.interval_index_of(&0), 0);
        assert_eq!(knots.interval_index_of(&5), 0);
        assert_eq!(knots.interval_index_of(&10), 1);
        assert_eq!(knots.interval_index_of(&15), 1);
        assert_eq!(knots.interval_index_of(&20), 1);
        assert_eq!(knots.interval_index_of(&25), 1);
    }
}
