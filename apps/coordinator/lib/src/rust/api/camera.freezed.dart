// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint, type=warning, deprecated_member_use, deprecated_member_use_from_same_package
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'camera.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$CameraEvent {

 Object get field0;



@override
bool operator ==(Object other) {
  final _this = this as CameraEvent;
  return identical(this, other) || (other.runtimeType == runtimeType&&other is CameraEvent&&const DeepCollectionEquality().equals(other.field0, _this.field0));
}


@override
int get hashCode {
  final _this = this as CameraEvent;
  return Object.hash(runtimeType,const DeepCollectionEquality().hash(_this.field0));
}

@override
String toString() {
  final _this = this as CameraEvent;
  return 'CameraEvent(field0: ${_this.field0})';
}


}

/// @nodoc
class $CameraEventCopyWith<$Res>  {
$CameraEventCopyWith(CameraEvent _, $Res Function(CameraEvent) __);
}


/// Adds pattern-matching-related methods to [CameraEvent].
extension CameraEventPatterns on CameraEvent {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( CameraEvent_Preview value)?  preview,TResult Function( CameraEvent_Qr value)?  qr,required TResult orElse(),}){
final _that = this;
switch (_that) {
case CameraEvent_Preview() when preview != null:
return preview(_that);case CameraEvent_Qr() when qr != null:
return qr(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( CameraEvent_Preview value)  preview,required TResult Function( CameraEvent_Qr value)  qr,}){
final _that = this;
switch (_that) {
case CameraEvent_Preview():
return preview(_that);case CameraEvent_Qr():
return qr(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( CameraEvent_Preview value)?  preview,TResult? Function( CameraEvent_Qr value)?  qr,}){
final _that = this;
switch (_that) {
case CameraEvent_Preview() when preview != null:
return preview(_that);case CameraEvent_Qr() when qr != null:
return qr(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( Uint8List field0)?  preview,TResult Function( String field0)?  qr,required TResult orElse(),}) {final _that = this;
switch (_that) {
case CameraEvent_Preview() when preview != null:
return preview(_that.field0);case CameraEvent_Qr() when qr != null:
return qr(_that.field0);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( Uint8List field0)  preview,required TResult Function( String field0)  qr,}) {final _that = this;
switch (_that) {
case CameraEvent_Preview():
return preview(_that.field0);case CameraEvent_Qr():
return qr(_that.field0);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( Uint8List field0)?  preview,TResult? Function( String field0)?  qr,}) {final _that = this;
switch (_that) {
case CameraEvent_Preview() when preview != null:
return preview(_that.field0);case CameraEvent_Qr() when qr != null:
return qr(_that.field0);case _:
  return null;

}
}

}

/// @nodoc


class CameraEvent_Preview extends CameraEvent {
  const CameraEvent_Preview(this.field0): super._();
  

@override final  Uint8List field0;

/// Create a copy of CameraEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$CameraEvent_PreviewCopyWith<CameraEvent_Preview> get copyWith => _$CameraEvent_PreviewCopyWithImpl<CameraEvent_Preview>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CameraEvent_Preview&&const DeepCollectionEquality().equals(other.field0, field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,const DeepCollectionEquality().hash(field0));
}

@override
String toString() {
    return 'CameraEvent.preview(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $CameraEvent_PreviewCopyWith<$Res> implements $CameraEventCopyWith<$Res> {
  factory $CameraEvent_PreviewCopyWith(CameraEvent_Preview value, $Res Function(CameraEvent_Preview) _then) = _$CameraEvent_PreviewCopyWithImpl;
@useResult
$Res call({
 Uint8List field0
});




}
/// @nodoc
class _$CameraEvent_PreviewCopyWithImpl<$Res>
    implements $CameraEvent_PreviewCopyWith<$Res> {
  _$CameraEvent_PreviewCopyWithImpl(this._self, this._then);

  final CameraEvent_Preview _self;
  final $Res Function(CameraEvent_Preview) _then;

/// Create a copy of CameraEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(CameraEvent_Preview(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as Uint8List,
  ));
}


}

/// @nodoc


class CameraEvent_Qr extends CameraEvent {
  const CameraEvent_Qr(this.field0): super._();
  

@override final  String field0;

/// Create a copy of CameraEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$CameraEvent_QrCopyWith<CameraEvent_Qr> get copyWith => _$CameraEvent_QrCopyWithImpl<CameraEvent_Qr>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CameraEvent_Qr&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'CameraEvent.qr(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $CameraEvent_QrCopyWith<$Res> implements $CameraEventCopyWith<$Res> {
  factory $CameraEvent_QrCopyWith(CameraEvent_Qr value, $Res Function(CameraEvent_Qr) _then) = _$CameraEvent_QrCopyWithImpl;
@useResult
$Res call({
 String field0
});




}
/// @nodoc
class _$CameraEvent_QrCopyWithImpl<$Res>
    implements $CameraEvent_QrCopyWith<$Res> {
  _$CameraEvent_QrCopyWithImpl(this._self, this._then);

  final CameraEvent_Qr _self;
  final $Res Function(CameraEvent_Qr) _then;

/// Create a copy of CameraEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(CameraEvent_Qr(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

// dart format on
