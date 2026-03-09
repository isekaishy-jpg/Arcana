intrinsic fn list_new[T]() -> List[T] = ListNew
intrinsic fn list_len[T](read xs: List[T]) -> Int = ListLen
intrinsic fn list_push[T](edit xs: List[T], take value: T) = ListPush
intrinsic fn list_pop[T](edit xs: List[T]) -> T = ListPop
intrinsic fn list_try_pop_or[T](edit xs: List[T], take fallback: T) -> (Bool, T) = ListTryPopOr

intrinsic fn array_new[T](len: Int, fill: T) -> Array[T] = ArrayNew
intrinsic fn array_len[T](read arr: Array[T]) -> Int = ArrayLen
intrinsic fn array_from_list[T](take xs: List[T]) -> Array[T] = ArrayFromList
intrinsic fn array_to_list[T](read arr: Array[T]) -> List[T] = ArrayToList

intrinsic fn map_new[K, V]() -> Map[K, V] = MapNew
intrinsic fn map_len[K, V](read m: Map[K, V]) -> Int = MapLen
intrinsic fn map_has[K, V](read m: Map[K, V], key: K) -> Bool = MapHas
intrinsic fn map_get[K, V](read m: Map[K, V], key: K) -> V = MapGet
intrinsic fn map_set[K, V](edit m: Map[K, V], key: K, take value: V) = MapSet
intrinsic fn map_remove[K, V](edit m: Map[K, V], key: K) -> Bool = MapRemove
intrinsic fn map_try_get_or[K, V](read m: Map[K, V], key: K, take fallback: V) -> (Bool, V) = MapTryGetOr
