// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint, type=warning, deprecated_member_use, deprecated_member_use_from_same_package
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'engine.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$Prompt {





@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Prompt);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'Prompt()';
}


}

/// @nodoc
class $PromptCopyWith<$Res>  {
$PromptCopyWith(Prompt _, $Res Function(Prompt) __);
}


/// Adds pattern-matching-related methods to [Prompt].
extension PromptPatterns on Prompt {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( Prompt_JoinKeygen value)?  joinKeygen,TResult Function( Prompt_VerifyKeygen value)?  verifyKeygen,TResult Function( Prompt_ReviewTransaction value)?  reviewTransaction,TResult Function( Prompt_SignTransaction value)?  signTransaction,required TResult orElse(),}){
final _that = this;
switch (_that) {
case Prompt_JoinKeygen() when joinKeygen != null:
return joinKeygen(_that);case Prompt_VerifyKeygen() when verifyKeygen != null:
return verifyKeygen(_that);case Prompt_ReviewTransaction() when reviewTransaction != null:
return reviewTransaction(_that);case Prompt_SignTransaction() when signTransaction != null:
return signTransaction(_that);case _:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( Prompt_JoinKeygen value)  joinKeygen,required TResult Function( Prompt_VerifyKeygen value)  verifyKeygen,required TResult Function( Prompt_ReviewTransaction value)  reviewTransaction,required TResult Function( Prompt_SignTransaction value)  signTransaction,}){
final _that = this;
switch (_that) {
case Prompt_JoinKeygen():
return joinKeygen(_that);case Prompt_VerifyKeygen():
return verifyKeygen(_that);case Prompt_ReviewTransaction():
return reviewTransaction(_that);case Prompt_SignTransaction():
return signTransaction(_that);}
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( Prompt_JoinKeygen value)?  joinKeygen,TResult? Function( Prompt_VerifyKeygen value)?  verifyKeygen,TResult? Function( Prompt_ReviewTransaction value)?  reviewTransaction,TResult? Function( Prompt_SignTransaction value)?  signTransaction,}){
final _that = this;
switch (_that) {
case Prompt_JoinKeygen() when joinKeygen != null:
return joinKeygen(_that);case Prompt_VerifyKeygen() when verifyKeygen != null:
return verifyKeygen(_that);case Prompt_ReviewTransaction() when reviewTransaction != null:
return reviewTransaction(_that);case Prompt_SignTransaction() when signTransaction != null:
return signTransaction(_that);case _:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( String keyName,  int threshold,  int nSigners,  String network)?  joinKeygen,TResult Function( String keyName,  int threshold,  int nSigners,  String checkCode)?  verifyKeygen,TResult Function( TxSummary field0)?  reviewTransaction,TResult Function( TxSummary field0)?  signTransaction,required TResult orElse(),}) {final _that = this;
switch (_that) {
case Prompt_JoinKeygen() when joinKeygen != null:
return joinKeygen(_that.keyName,_that.threshold,_that.nSigners,_that.network);case Prompt_VerifyKeygen() when verifyKeygen != null:
return verifyKeygen(_that.keyName,_that.threshold,_that.nSigners,_that.checkCode);case Prompt_ReviewTransaction() when reviewTransaction != null:
return reviewTransaction(_that.field0);case Prompt_SignTransaction() when signTransaction != null:
return signTransaction(_that.field0);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( String keyName,  int threshold,  int nSigners,  String network)  joinKeygen,required TResult Function( String keyName,  int threshold,  int nSigners,  String checkCode)  verifyKeygen,required TResult Function( TxSummary field0)  reviewTransaction,required TResult Function( TxSummary field0)  signTransaction,}) {final _that = this;
switch (_that) {
case Prompt_JoinKeygen():
return joinKeygen(_that.keyName,_that.threshold,_that.nSigners,_that.network);case Prompt_VerifyKeygen():
return verifyKeygen(_that.keyName,_that.threshold,_that.nSigners,_that.checkCode);case Prompt_ReviewTransaction():
return reviewTransaction(_that.field0);case Prompt_SignTransaction():
return signTransaction(_that.field0);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( String keyName,  int threshold,  int nSigners,  String network)?  joinKeygen,TResult? Function( String keyName,  int threshold,  int nSigners,  String checkCode)?  verifyKeygen,TResult? Function( TxSummary field0)?  reviewTransaction,TResult? Function( TxSummary field0)?  signTransaction,}) {final _that = this;
switch (_that) {
case Prompt_JoinKeygen() when joinKeygen != null:
return joinKeygen(_that.keyName,_that.threshold,_that.nSigners,_that.network);case Prompt_VerifyKeygen() when verifyKeygen != null:
return verifyKeygen(_that.keyName,_that.threshold,_that.nSigners,_that.checkCode);case Prompt_ReviewTransaction() when reviewTransaction != null:
return reviewTransaction(_that.field0);case Prompt_SignTransaction() when signTransaction != null:
return signTransaction(_that.field0);case _:
  return null;

}
}

}

/// @nodoc


class Prompt_JoinKeygen extends Prompt {
  const Prompt_JoinKeygen({required this.keyName, required this.threshold, required this.nSigners, required this.network}): super._();
  

 final  String keyName;
 final  int threshold;
 final  int nSigners;
 final  String network;

/// Create a copy of Prompt
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Prompt_JoinKeygenCopyWith<Prompt_JoinKeygen> get copyWith => _$Prompt_JoinKeygenCopyWithImpl<Prompt_JoinKeygen>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Prompt_JoinKeygen&&(identical(other.keyName, keyName) || other.keyName == keyName)&&(identical(other.threshold, threshold) || other.threshold == threshold)&&(identical(other.nSigners, nSigners) || other.nSigners == nSigners)&&(identical(other.network, network) || other.network == network));
}


@override
int get hashCode {
    return Object.hash(runtimeType,keyName,threshold,nSigners,network);
}

@override
String toString() {
    return 'Prompt.joinKeygen(keyName: $keyName, threshold: $threshold, nSigners: $nSigners, network: $network)';
}


}

/// @nodoc
abstract mixin class $Prompt_JoinKeygenCopyWith<$Res> implements $PromptCopyWith<$Res> {
  factory $Prompt_JoinKeygenCopyWith(Prompt_JoinKeygen value, $Res Function(Prompt_JoinKeygen) _then) = _$Prompt_JoinKeygenCopyWithImpl;
@useResult
$Res call({
 String keyName, int threshold, int nSigners, String network
});




}
/// @nodoc
class _$Prompt_JoinKeygenCopyWithImpl<$Res>
    implements $Prompt_JoinKeygenCopyWith<$Res> {
  _$Prompt_JoinKeygenCopyWithImpl(this._self, this._then);

  final Prompt_JoinKeygen _self;
  final $Res Function(Prompt_JoinKeygen) _then;

/// Create a copy of Prompt
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? keyName = null,Object? threshold = null,Object? nSigners = null,Object? network = null,}) {
  return _then(Prompt_JoinKeygen(
keyName: null == keyName ? _self.keyName : keyName // ignore: cast_nullable_to_non_nullable
as String,threshold: null == threshold ? _self.threshold : threshold // ignore: cast_nullable_to_non_nullable
as int,nSigners: null == nSigners ? _self.nSigners : nSigners // ignore: cast_nullable_to_non_nullable
as int,network: null == network ? _self.network : network // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class Prompt_VerifyKeygen extends Prompt {
  const Prompt_VerifyKeygen({required this.keyName, required this.threshold, required this.nSigners, required this.checkCode}): super._();
  

 final  String keyName;
 final  int threshold;
 final  int nSigners;
 final  String checkCode;

/// Create a copy of Prompt
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Prompt_VerifyKeygenCopyWith<Prompt_VerifyKeygen> get copyWith => _$Prompt_VerifyKeygenCopyWithImpl<Prompt_VerifyKeygen>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Prompt_VerifyKeygen&&(identical(other.keyName, keyName) || other.keyName == keyName)&&(identical(other.threshold, threshold) || other.threshold == threshold)&&(identical(other.nSigners, nSigners) || other.nSigners == nSigners)&&(identical(other.checkCode, checkCode) || other.checkCode == checkCode));
}


@override
int get hashCode {
    return Object.hash(runtimeType,keyName,threshold,nSigners,checkCode);
}

@override
String toString() {
    return 'Prompt.verifyKeygen(keyName: $keyName, threshold: $threshold, nSigners: $nSigners, checkCode: $checkCode)';
}


}

/// @nodoc
abstract mixin class $Prompt_VerifyKeygenCopyWith<$Res> implements $PromptCopyWith<$Res> {
  factory $Prompt_VerifyKeygenCopyWith(Prompt_VerifyKeygen value, $Res Function(Prompt_VerifyKeygen) _then) = _$Prompt_VerifyKeygenCopyWithImpl;
@useResult
$Res call({
 String keyName, int threshold, int nSigners, String checkCode
});




}
/// @nodoc
class _$Prompt_VerifyKeygenCopyWithImpl<$Res>
    implements $Prompt_VerifyKeygenCopyWith<$Res> {
  _$Prompt_VerifyKeygenCopyWithImpl(this._self, this._then);

  final Prompt_VerifyKeygen _self;
  final $Res Function(Prompt_VerifyKeygen) _then;

/// Create a copy of Prompt
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? keyName = null,Object? threshold = null,Object? nSigners = null,Object? checkCode = null,}) {
  return _then(Prompt_VerifyKeygen(
keyName: null == keyName ? _self.keyName : keyName // ignore: cast_nullable_to_non_nullable
as String,threshold: null == threshold ? _self.threshold : threshold // ignore: cast_nullable_to_non_nullable
as int,nSigners: null == nSigners ? _self.nSigners : nSigners // ignore: cast_nullable_to_non_nullable
as int,checkCode: null == checkCode ? _self.checkCode : checkCode // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class Prompt_ReviewTransaction extends Prompt {
  const Prompt_ReviewTransaction(this.field0): super._();
  

 final  TxSummary field0;

/// Create a copy of Prompt
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Prompt_ReviewTransactionCopyWith<Prompt_ReviewTransaction> get copyWith => _$Prompt_ReviewTransactionCopyWithImpl<Prompt_ReviewTransaction>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Prompt_ReviewTransaction&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Prompt.reviewTransaction(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Prompt_ReviewTransactionCopyWith<$Res> implements $PromptCopyWith<$Res> {
  factory $Prompt_ReviewTransactionCopyWith(Prompt_ReviewTransaction value, $Res Function(Prompt_ReviewTransaction) _then) = _$Prompt_ReviewTransactionCopyWithImpl;
@useResult
$Res call({
 TxSummary field0
});




}
/// @nodoc
class _$Prompt_ReviewTransactionCopyWithImpl<$Res>
    implements $Prompt_ReviewTransactionCopyWith<$Res> {
  _$Prompt_ReviewTransactionCopyWithImpl(this._self, this._then);

  final Prompt_ReviewTransaction _self;
  final $Res Function(Prompt_ReviewTransaction) _then;

/// Create a copy of Prompt
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Prompt_ReviewTransaction(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as TxSummary,
  ));
}


}

/// @nodoc


class Prompt_SignTransaction extends Prompt {
  const Prompt_SignTransaction(this.field0): super._();
  

 final  TxSummary field0;

/// Create a copy of Prompt
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Prompt_SignTransactionCopyWith<Prompt_SignTransaction> get copyWith => _$Prompt_SignTransactionCopyWithImpl<Prompt_SignTransaction>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Prompt_SignTransaction&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Prompt.signTransaction(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Prompt_SignTransactionCopyWith<$Res> implements $PromptCopyWith<$Res> {
  factory $Prompt_SignTransactionCopyWith(Prompt_SignTransaction value, $Res Function(Prompt_SignTransaction) _then) = _$Prompt_SignTransactionCopyWithImpl;
@useResult
$Res call({
 TxSummary field0
});




}
/// @nodoc
class _$Prompt_SignTransactionCopyWithImpl<$Res>
    implements $Prompt_SignTransactionCopyWith<$Res> {
  _$Prompt_SignTransactionCopyWithImpl(this._self, this._then);

  final Prompt_SignTransaction _self;
  final $Res Function(Prompt_SignTransaction) _then;

/// Create a copy of Prompt
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Prompt_SignTransaction(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as TxSummary,
  ));
}


}

/// @nodoc
mixin _$Step {





@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Step);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'Step()';
}


}

/// @nodoc
class $StepCopyWith<$Res>  {
$StepCopyWith(Step _, $Res Function(Step) __);
}


/// Adds pattern-matching-related methods to [Step].
extension StepPatterns on Step {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( Step_Ignored value)?  ignored,TResult Function( Step_Confirm value)?  confirm,TResult Function( Step_Reply value)?  reply,TResult Function( Step_Done value)?  done,required TResult orElse(),}){
final _that = this;
switch (_that) {
case Step_Ignored() when ignored != null:
return ignored(_that);case Step_Confirm() when confirm != null:
return confirm(_that);case Step_Reply() when reply != null:
return reply(_that);case Step_Done() when done != null:
return done(_that);case _:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( Step_Ignored value)  ignored,required TResult Function( Step_Confirm value)  confirm,required TResult Function( Step_Reply value)  reply,required TResult Function( Step_Done value)  done,}){
final _that = this;
switch (_that) {
case Step_Ignored():
return ignored(_that);case Step_Confirm():
return confirm(_that);case Step_Reply():
return reply(_that);case Step_Done():
return done(_that);}
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( Step_Ignored value)?  ignored,TResult? Function( Step_Confirm value)?  confirm,TResult? Function( Step_Reply value)?  reply,TResult? Function( Step_Done value)?  done,}){
final _that = this;
switch (_that) {
case Step_Ignored() when ignored != null:
return ignored(_that);case Step_Confirm() when confirm != null:
return confirm(_that);case Step_Reply() when reply != null:
return reply(_that);case Step_Done() when done != null:
return done(_that);case _:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( String reason)?  ignored,TResult Function( Prompt field0)?  confirm,TResult Function( List<String> frames,  String what)?  reply,TResult Function( String message)?  done,required TResult orElse(),}) {final _that = this;
switch (_that) {
case Step_Ignored() when ignored != null:
return ignored(_that.reason);case Step_Confirm() when confirm != null:
return confirm(_that.field0);case Step_Reply() when reply != null:
return reply(_that.frames,_that.what);case Step_Done() when done != null:
return done(_that.message);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( String reason)  ignored,required TResult Function( Prompt field0)  confirm,required TResult Function( List<String> frames,  String what)  reply,required TResult Function( String message)  done,}) {final _that = this;
switch (_that) {
case Step_Ignored():
return ignored(_that.reason);case Step_Confirm():
return confirm(_that.field0);case Step_Reply():
return reply(_that.frames,_that.what);case Step_Done():
return done(_that.message);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( String reason)?  ignored,TResult? Function( Prompt field0)?  confirm,TResult? Function( List<String> frames,  String what)?  reply,TResult? Function( String message)?  done,}) {final _that = this;
switch (_that) {
case Step_Ignored() when ignored != null:
return ignored(_that.reason);case Step_Confirm() when confirm != null:
return confirm(_that.field0);case Step_Reply() when reply != null:
return reply(_that.frames,_that.what);case Step_Done() when done != null:
return done(_that.message);case _:
  return null;

}
}

}

/// @nodoc


class Step_Ignored extends Step {
  const Step_Ignored({required this.reason}): super._();
  

 final  String reason;

/// Create a copy of Step
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Step_IgnoredCopyWith<Step_Ignored> get copyWith => _$Step_IgnoredCopyWithImpl<Step_Ignored>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Step_Ignored&&(identical(other.reason, reason) || other.reason == reason));
}


@override
int get hashCode {
    return Object.hash(runtimeType,reason);
}

@override
String toString() {
    return 'Step.ignored(reason: $reason)';
}


}

/// @nodoc
abstract mixin class $Step_IgnoredCopyWith<$Res> implements $StepCopyWith<$Res> {
  factory $Step_IgnoredCopyWith(Step_Ignored value, $Res Function(Step_Ignored) _then) = _$Step_IgnoredCopyWithImpl;
@useResult
$Res call({
 String reason
});




}
/// @nodoc
class _$Step_IgnoredCopyWithImpl<$Res>
    implements $Step_IgnoredCopyWith<$Res> {
  _$Step_IgnoredCopyWithImpl(this._self, this._then);

  final Step_Ignored _self;
  final $Res Function(Step_Ignored) _then;

/// Create a copy of Step
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? reason = null,}) {
  return _then(Step_Ignored(
reason: null == reason ? _self.reason : reason // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class Step_Confirm extends Step {
  const Step_Confirm(this.field0): super._();
  

 final  Prompt field0;

/// Create a copy of Step
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Step_ConfirmCopyWith<Step_Confirm> get copyWith => _$Step_ConfirmCopyWithImpl<Step_Confirm>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Step_Confirm&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Step.confirm(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Step_ConfirmCopyWith<$Res> implements $StepCopyWith<$Res> {
  factory $Step_ConfirmCopyWith(Step_Confirm value, $Res Function(Step_Confirm) _then) = _$Step_ConfirmCopyWithImpl;
@useResult
$Res call({
 Prompt field0
});


$PromptCopyWith<$Res> get field0;

}
/// @nodoc
class _$Step_ConfirmCopyWithImpl<$Res>
    implements $Step_ConfirmCopyWith<$Res> {
  _$Step_ConfirmCopyWithImpl(this._self, this._then);

  final Step_Confirm _self;
  final $Res Function(Step_Confirm) _then;

/// Create a copy of Step
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Step_Confirm(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as Prompt,
  ));
}

/// Create a copy of Step
/// with the given fields replaced by the non-null parameter values.
@override
@pragma('vm:prefer-inline')
$PromptCopyWith<$Res> get field0 {
  
  return $PromptCopyWith<$Res>(_self.field0, (value) {
    return _then(_self.copyWith(field0: value));
  });
}
}

/// @nodoc


class Step_Reply extends Step {
  const Step_Reply({required  List<String> frames, required this.what}): _frames = frames,super._();
  

 final  List<String> _frames;
 List<String> get frames {
  if (_frames is EqualUnmodifiableListView) return _frames;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_frames);
}

 final  String what;

/// Create a copy of Step
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Step_ReplyCopyWith<Step_Reply> get copyWith => _$Step_ReplyCopyWithImpl<Step_Reply>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Step_Reply&&const DeepCollectionEquality().equals(other.frames, _frames)&&(identical(other.what, what) || other.what == what));
}


@override
int get hashCode {
    return Object.hash(runtimeType,const DeepCollectionEquality().hash(_frames),what);
}

@override
String toString() {
    return 'Step.reply(frames: $frames, what: $what)';
}


}

/// @nodoc
abstract mixin class $Step_ReplyCopyWith<$Res> implements $StepCopyWith<$Res> {
  factory $Step_ReplyCopyWith(Step_Reply value, $Res Function(Step_Reply) _then) = _$Step_ReplyCopyWithImpl;
@useResult
$Res call({
 List<String> frames, String what
});




}
/// @nodoc
class _$Step_ReplyCopyWithImpl<$Res>
    implements $Step_ReplyCopyWith<$Res> {
  _$Step_ReplyCopyWithImpl(this._self, this._then);

  final Step_Reply _self;
  final $Res Function(Step_Reply) _then;

/// Create a copy of Step
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? frames = null,Object? what = null,}) {
  return _then(Step_Reply(
frames: null == frames ? _self._frames : frames // ignore: cast_nullable_to_non_nullable
as List<String>,what: null == what ? _self.what : what // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class Step_Done extends Step {
  const Step_Done({required this.message}): super._();
  

 final  String message;

/// Create a copy of Step
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Step_DoneCopyWith<Step_Done> get copyWith => _$Step_DoneCopyWithImpl<Step_Done>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Step_Done&&(identical(other.message, message) || other.message == message));
}


@override
int get hashCode {
    return Object.hash(runtimeType,message);
}

@override
String toString() {
    return 'Step.done(message: $message)';
}


}

/// @nodoc
abstract mixin class $Step_DoneCopyWith<$Res> implements $StepCopyWith<$Res> {
  factory $Step_DoneCopyWith(Step_Done value, $Res Function(Step_Done) _then) = _$Step_DoneCopyWithImpl;
@useResult
$Res call({
 String message
});




}
/// @nodoc
class _$Step_DoneCopyWithImpl<$Res>
    implements $Step_DoneCopyWith<$Res> {
  _$Step_DoneCopyWithImpl(this._self, this._then);

  final Step_Done _self;
  final $Res Function(Step_Done) _then;

/// Create a copy of Step
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? message = null,}) {
  return _then(Step_Done(
message: null == message ? _self.message : message // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

// dart format on
